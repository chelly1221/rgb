//! GS3087T lighting only; protocol recovered from its official 20240923 driver.
use super::Result;
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::{thread::sleep, time::Duration};
fn matches(d: &DeviceInfo) -> bool {
    d.vendor_id() == 0x05ac
        && d.product_id() == 0x0256
        && d.interface_number() == 0
        && d.usage_page() == 1
        && d.usage() == 6
        && d.product_string() == Some("GS3087T")
}
pub fn present(api: &HidApi) -> bool {
    api.device_list().any(matches)
}
pub struct Keyboard(HidDevice);
impl Keyboard {
    pub fn open(api: &HidApi) -> Result<Self> {
        let devices: Vec<_> = api.device_list().filter(|d| matches(d)).collect();
        if devices.len() != 1 {
            return Err("GS3087T 유선 설정 인터페이스를 하나로 식별하지 못했습니다.".into());
        }
        let d = devices[0].open_device(api).map_err(|e| e.to_string())?;
        let mut descriptor = [0; 4096];
        let n = d
            .get_report_descriptor(&mut descriptor)
            .map_err(|e| e.to_string())?;
        if n != 95 || !descriptor[..n].ends_with(&[0x75, 8, 0x95, 0x40, 0xb1, 2, 0xc0]) {
            return Err("검증한 GS3087T 설정 보고서 형식과 다릅니다.".into());
        }
        Ok(Self(d))
    }
    fn send(&self, p: &[u8; 65]) -> Result<()> {
        self.0
            .send_feature_report(p)
            .map_err(|e| format!("키보드 조명 전송 실패: {e}"))?;
        sleep(Duration::from_millis(20));
        Ok(())
    }
    fn command(&self, opcode: u8, blocks: u8) -> Result<()> {
        let p = command_packet(opcode, blocks);
        self.send(&p)?;
        let mut last = [0; 8];
        for attempt in 0..4 {
            let mut r = p;
            let n = self
                .0
                .get_feature_report(&mut r)
                .map_err(|e| e.to_string())?;
            last.copy_from_slice(&r[..8]);
            if acknowledged(&r[..n], opcode) {
                return Ok(());
            }
            // The official driver repeats a command once when firmware returns FF (busy).
            if n == 65 && r[1] == 4 && r[4] == 0xff && attempt == 0 {
                self.send(&p)?;
            }
            sleep(Duration::from_millis(20));
        }
        Err(format!(
            "키보드가 조명 명령 {opcode:02X}을 승인하지 않았습니다: {last:02x?}"
        ))
    }
    pub fn power(&self, enabled: bool) -> Result<()> {
        self.command(0x18, 0)?;
        let result: Result<()> = (|| {
            self.command(0x13, 1)?;
            self.send(&lighting_packet(enabled))?;
            self.command(0x02, 0)?;
            sleep(Duration::from_millis(150)); // allow the firmware's rendered frame to settle
            let colors = self.colors()?;
            let valid = if enabled {
                colors.iter().filter(|c| **c != [0; 3]).count() == 88
                    && colors.iter().all(|c| *c == [0; 3] || *c == [0xfd; 3])
            } else {
                colors.iter().all(|c| *c == [0; 3])
            };
            if !valid {
                return Err(format!("키보드 색상 재조회가 요청과 다릅니다 (흰색 {} / 점등 {}). 다른 조명 프로그램을 확인하세요.",colors.iter().filter(|c|**c==[0xfd;3]).count(),colors.iter().filter(|c|**c!=[0;3]).count()));
            }
            Ok(())
        })();
        if let Err(e) = result {
            let close = self.command(0x02, 0);
            return Err(format!(
                "{e} 조명 적용 여부를 확인하지 못했습니다. 설정 전송 종료: {}",
                if close.is_ok() { "완료" } else { "실패" }
            ));
        }
        // Omit the vendor's F0 flash-save command for a temporary RGB switch.
        Ok(())
    }
    pub fn colors(&self) -> Result<Vec<[u8; 3]>> {
        self.send(&command_packet(0xf5, 9))?;
        let read: Result<Vec<[u8; 3]>> = (|| {
            let mut colors = Vec::with_capacity(144);
            for block in 0..9 {
                sleep(Duration::from_millis(20));
                let mut r = [0; 65];
                let n = self
                    .0
                    .get_feature_report(&mut r)
                    .map_err(|e| e.to_string())?;
                if n != 65 || r[0] != 0 {
                    return Err("키보드 색상 응답 길이가 다릅니다.".into());
                }
                for (i, entry) in r[1..].chunks_exact(4).enumerate() {
                    if entry[0] as usize != block * 16 + i {
                        return Err("키보드 색상 응답 순서가 다릅니다.".into());
                    }
                    colors.push([entry[1], entry[2], entry[3]]);
                }
            }
            Ok(colors)
        })();
        let close = self.command(0x02, 0);
        let colors = read?;
        close?;
        Ok(colors)
    }
    pub fn lighting(&self, settings: &super::lighting::Lighting) -> Result<()> {
        settings.validate()?;
        self.command(0x18, 0)?;
        let write = (|| {
            self.command(0x13, 1)?;
            self.send(&effect_packet(settings))?;
            self.command(0x02, 0)
        })();
        if write.is_err() {
            let _ = self.command(0x02, 0);
        }
        write
    }
}
fn effect_packet(s: &super::lighting::Lighting) -> [u8; 65] {
    use super::lighting::Effect;
    let mut p = lighting_packet(true);
    p[1] = if s.brightness == 0 {
        0
    } else {
        match s.effect {
            Effect::Static => 1,
            Effect::Breathe => 7,
            Effect::Spectrum => 8,
        }
    };
    p[12] = p[1];
    p[2..5].copy_from_slice(&s.color);
    p[10] = s.level(15) + 1;
    p[11] = match s.speed {
        1 => 4,
        2 => 9,
        _ => 16,
    };
    p
}
fn acknowledged(r: &[u8], opcode: u8) -> bool {
    r.len() == 65 && r[0] == 0 && r[1] == 4 && r[2] == opcode && r[4] == 1
}
fn command_packet(opcode: u8, blocks: u8) -> [u8; 65] {
    let mut p = [0; 65];
    p[1] = 4;
    p[2] = opcode;
    p[9] = blocks;
    p
}
fn lighting_packet(enabled: bool) -> [u8; 65] {
    let mut p = [0; 65];
    p[1] = u8::from(enabled); // 0=off, 1=static in the exact-model device.xml.
    p[2..5].fill(255);
    p[9] = 0; // single selected color
    p[10] = 16; // brightness + 1; 15 is the device maximum
    p[11] = 11; // default speed + 1
    p[12] = u8::from(enabled); // active effect ID, independently repeated after settings
    p[15] = 0xaa;
    p[16] = 0x55;
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_short_and_busy_replies_are_not_success() {
        let mut r = command_packet(0x18, 0);
        r[4] = 1;
        assert!(acknowledged(&r, 0x18));
        assert!(!acknowledged(&r, 0x02));
        assert!(!acknowledged(&r[..10], 0x18));
        r[4] = 0xff;
        assert!(!acknowledged(&r, 0x18));
    }
    #[test]
    fn lighting_does_not_contain_keymap_or_flash_commands() {
        for on in [false, true] {
            let p = lighting_packet(on);
            assert_eq!(p[0], 0);
            assert_eq!(p[1], u8::from(on));
            assert_eq!(p[12], u8::from(on)); // missing active effect leaves the old color running
            assert_eq!(&p[15..17], &[0xaa, 0x55]);
            assert!(p[17..].iter().all(|b| *b == 0));
        }
    }
    #[test]
    fn native_effects_repeat_the_active_id_and_respect_zero_brightness() {
        use super::super::lighting::{Effect, Lighting};
        for (effect, id) in [
            (Effect::Static, 1),
            (Effect::Breathe, 7),
            (Effect::Spectrum, 8),
        ] {
            let mut s = Lighting {
                effect,
                color: [255, 90, 140],
                brightness: 60,
                speed: 2,
            };
            let p = effect_packet(&s);
            assert_eq!((p[1], p[12]), (id, id));
            assert_eq!(&p[2..5], &s.color);
            assert_eq!(p[10], 10);
            assert!(p[17..].iter().all(|v| *v == 0));
            s.brightness = 0;
            assert_eq!((effect_packet(&s)[1], effect_packet(&s)[12]), (0, 0));
        }
    }
}
