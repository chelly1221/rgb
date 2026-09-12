//! HARPOON 1B1C:1B5E, wired vendor interface only. No receiver routing.
use super::Result;
use hidapi::{DeviceInfo, HidApi, HidDevice};

pub fn matches(info: &DeviceInfo) -> bool {
    info.vendor_id() == 0x1b1c
        && info.product_id() == 0x1b5e
        && info.interface_number() == 1
        && info.usage_page() == 0xff42
        && info.usage() == 1
}

pub struct Mouse {
    device: HidDevice,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DpiProfile {
    pub values: [u16; 5],
    pub colors: [[u8; 3]; 5],
    pub mask: u8,
    pub index: u8,
}
impl DpiProfile {
    pub fn validate(&self) -> Result<()> {
        next_dpi(self.mask, self.index)?;
        if self.mask & (1 << self.index) == 0
            || self
                .values
                .iter()
                .any(|v| !(200..=10000).contains(v) || v % 200 != 0)
        {
            return Err(
                "DPI는 200~10000 사이의 200 단위로, 현재 단계는 활성 단계로 선택하세요.".into(),
            );
        }
        Ok(())
    }
}
pub struct DpiListener(HidDevice, bool);
impl DpiListener {
    pub fn open(api: &HidApi) -> Result<Self> {
        let candidates: Vec<_> = api
            .device_list()
            .filter(|d| {
                d.vendor_id() == 0x1b1c
                    && d.product_id() == 0x1b5e
                    && d.interface_number() == 2
                    && d.usage_page() == 0xff42
                    && d.usage() == 2
            })
            .collect();
        if candidates.len() != 1 {
            return Err("마우스 DPI 버튼 인터페이스를 식별하지 못했습니다.".into());
        }
        Ok(Self(
            candidates[0].open_device(api).map_err(|e| e.to_string())?,
            false,
        ))
    }
    pub fn pressed(&mut self) -> Result<bool> {
        let mut input = [0; 65];
        let n = self
            .0
            .read_timeout(&mut input, 1)
            .map_err(|e| e.to_string())?;
        Ok(dpi_pressed(&input[..n], &mut self.1))
    }
}
fn dpi_pressed(input: &[u8], held: &mut bool) -> bool {
    if input.len() < 3 || input[0] != 0 || input[1] != 2 {
        return false;
    }
    let down = input[2] & 8 != 0;
    let pressed = down && !*held;
    *held = down;
    pressed
}
pub fn next_dpi(mask: u8, index: u8) -> Result<u8> {
    if mask == 0 || mask & !0x1f != 0 || index > 4 {
        return Err("저장된 DPI 단계 형식이 다릅니다.".into());
    }
    (1..=5)
        .map(|offset| (index + offset) % 5)
        .find(|next| mask & (1 << next) != 0)
        .ok_or_else(|| "사용 가능한 DPI 단계가 없습니다.".into())
}
impl Mouse {
    pub fn open(api: &HidApi) -> Result<Self> {
        let candidates: Vec<_> = api.device_list().filter(|d| matches(d)).collect();
        if candidates.len() != 1 {
            return Err(
                "HARPOON 마우스를 USB 케이블로 연결하세요. 무선 수신기는 아직 지원하지 않습니다."
                    .into(),
            );
        }
        let mouse = Self {
            device: candidates[0].open_device(api).map_err(|e| e.to_string())?,
        };
        // Drain only this vendor interface, never the mouse movement/keyboard endpoints.
        for _ in 0..16 {
            let mut stale = [0; 65];
            if mouse
                .device
                .read_timeout(&mut stale, 2)
                .map_err(|e| e.to_string())?
                == 0
            {
                break;
            }
        }
        let reply = mouse.exchange(packet(2, 0x12, 0, 0))?;
        if u16::from_le_bytes([reply[3], reply[4]]) != 0x1b5e {
            return Err("마우스 내부 제품 ID가 일치하지 않습니다.".into());
        }
        Ok(mouse)
    }

    fn exchange(&self, packet: [u8; 65]) -> Result<Vec<u8>> {
        let written = self
            .device
            .write(&packet)
            .map_err(|e| format!("마우스 전송 실패: {e}"))?;
        if written != packet.len() {
            return Err("마우스에 패킷을 일부만 전송했습니다.".into());
        }
        let mut response = [0; 65];
        let n = self
            .device
            .read_timeout(&mut response, 500)
            .map_err(|e| e.to_string())?;
        validate_reply(packet[2], &response[..n]).map_err(|e| {
            format!(
                "{e} [명령 {:02X}, 항목 {:02X}, 옵션 {:02X}]",
                packet[2], packet[3], packet[4]
            )
        })?;
        Ok(response[..n].to_vec())
    }

    pub fn render_mode(&self) -> Result<u8> {
        Ok(self.exchange(packet(2, 3, 0, 0))?[3])
    }
    pub fn inspect_lighting(&self) -> Vec<(u8, Result<Vec<u8>>)> {
        [
            2, 0x1e, 0x1f, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x2f, 0x30, 0x31, 0x32, 0x33,
        ]
        .into_iter()
        .map(|property| (property, self.exchange(packet(2, property, 0, 0))))
        .collect()
    }
    pub fn dpi_state(&self) -> Result<(u8, [u8; 3])> {
        let index = self.exchange(packet(2, 0x1e, 0, 0))?[3];
        if index > 4 {
            return Err(format!("검증 범위 밖의 DPI 단계: {index}"));
        }
        let reply = self.exchange(packet(2, 0x2f + index, 0, 0))?;
        // Stored stage colors use BGR; the software lighting stream uses RGB planes.
        Ok((index, [reply[5], reply[4], reply[3]]))
    }
    pub fn current_dpi(&self) -> Result<u16> {
        let reply = self.exchange(packet(2, 0x20, 0, 0))?;
        Ok(u16::from_le_bytes([reply[3], reply[4]]))
    }
    pub fn dpi_profile(&self) -> Result<DpiProfile> {
        let index = self.exchange(packet(2, 0x1e, 0, 0))?[3];
        let mask = self.exchange(packet(2, 0x1f, 0, 0))?[3];
        next_dpi(mask, index)?;
        let mut profile = DpiProfile {
            values: [0; 5],
            colors: [[0; 3]; 5],
            mask,
            index,
        };
        for i in 0..5 {
            let value = self.exchange(packet(2, 0x18 + i as u8, 0, 0))?;
            let dpi = u16::from_le_bytes([value[3], value[4]]);
            if !(100..=10000).contains(&dpi) {
                return Err(format!("DPI {}단계의 값이 검증 범위 밖입니다.", i + 1));
            }
            let color = self.exchange(packet(2, 0x2f + i as u8, 0, 0))?;
            profile.values[i] = dpi;
            profile.colors[i] = [color[5], color[4], color[3]];
        }
        Ok(profile)
    }
    pub fn select_dpi(&self, profile: &DpiProfile, index: u8) -> Result<()> {
        profile.validate()?;
        if index > 4 || profile.mask & (1 << index) == 0 {
            return Err("비활성 DPI 단계입니다.".into());
        }
        let old = self.exchange(packet(2, 0x1e, 0, 0))?[3];
        if old > 4 {
            return Err("현재 DPI 단계 형식이 다릅니다.".into());
        }
        let old_value = self.current_dpi()?;
        let result = self.set_current_dpi(profile.values[index as usize], index);
        if let Err(error) = result {
            let restore = self.set_current_dpi(old_value, old);
            return Err(format!("{error} 이전 DPI 단계 복원: {restore:?}"));
        }
        Ok(())
    }
    fn set_current_dpi(&self, value: u16, index: u8) -> Result<()> {
        let mut current = packet(1, 0x20, 0, 0);
        current[5..7].copy_from_slice(&value.to_le_bytes());
        self.exchange(current)?;
        self.exchange(packet(1, 0x1e, 0, index))?;
        if self.exchange(packet(2, 0x1e, 0, 0))?[3] != index || self.current_dpi()? != value {
            return Err("DPI 단계 또는 감도 재조회가 일치하지 않습니다.".into());
        }
        Ok(())
    }
    pub fn heartbeat(&self) -> Result<()> {
        self.exchange(packet(0x12, 0, 0, 0)).map(|_| ())
    }
    pub fn finish_software(&self, enabled: bool) -> Result<()> {
        let _ = self.exchange(packet(5, 1, 0, 0));
        let result = self.power(enabled);
        if result.is_err() {
            let _ = self.exchange(packet(1, 3, 0, 1));
        }
        result
    }
    pub fn begin_software_rgb(&self) -> Result<()> {
        self.exchange(packet(1, 3, 0, 2))?;
        self.exchange(brightness_packet(1000))?;
        if self.exchange(packet(0x0d, 0, 1, 0)).is_err() {
            self.exchange(packet(5, 1, 0, 0))?;
            self.exchange(packet(0x0d, 0, 1, 0))?;
        }
        Ok(())
    }
    pub fn software_frame(&self, logo: [u8; 3], indicator: [u8; 3]) -> Result<()> {
        let mut frame = [0; 65];
        frame[1] = 8;
        frame[2] = 6;
        frame[4] = 6;
        frame[8..14].copy_from_slice(&[
            indicator[0],
            logo[0],
            indicator[1],
            logo[1],
            indicator[2],
            logo[2],
        ]);
        self.exchange(frame)?;
        Ok(())
    }

    pub fn power(&self, enabled: bool) -> Result<()> {
        let old_mode = self.render_mode()?;
        let old_brightness = self.brightness()?;
        let dpi = self.exchange(packet(2, 0x1e, 0, 0))?[3];
        let result: Result<()> = (|| {
            // Onboard mode retains DPI-button processing and the stored stage colors.
            // RGB streaming is handled separately with a live DPI-button listener.
            // Brightness writes are permitted only while software mode is active.
            self.exchange(packet(1, 3, 0, 2))?;
            self.exchange(brightness_packet(if enabled { 1000 } else { 0 }))?;
            self.exchange(packet(1, 3, 0, 1))?;
            if self.render_mode()? != 1 || self.brightness()? != if enabled { 1000 } else { 0 } {
                return Err(
                    "마우스 기본 조명 모드·밝기 재조회가 다릅니다. 다른 조명 앱을 확인하세요."
                        .into(),
                );
            }
            let after = self.exchange(packet(2, 0x1e, 0, 0))?[3];
            if dpi != after {
                return Err("적용 중 DPI 단계가 바뀌었습니다. DPI 버튼을 누르지 않은 상태에서 다시 시도하세요.".into());
            }
            Ok(())
        })();
        if let Err(e) = result {
            let _ = self.exchange(packet(1, 3, 0, 2));
            let brightness = self.exchange(brightness_packet(old_brightness));
            let mode = self.exchange(packet(1, 3, 0, old_mode));
            return Err(if brightness.is_ok() && mode.is_ok() {
                format!("{e} 이전 설정을 복원했습니다.")
            } else {
                format!("{e} 이전 조명 모드 복원도 실패했습니다.")
            });
        }
        Ok(())
    }
    fn brightness(&self) -> Result<u16> {
        let reply = self.exchange(packet(2, 2, 0, 0))?;
        let value = u16::from_le_bytes([reply[3], reply[4]]);
        if value > 1000 {
            return Err("마우스 밝기 범위가 검증한 형식과 다릅니다.".into());
        }
        Ok(value)
    }
}

fn packet(command: u8, property: u8, option: u8, value: u8) -> [u8; 65] {
    let mut p = [0; 65];
    p[1] = 8;
    p[2] = command;
    p[3] = property;
    p[4] = option;
    p[5] = value;
    p
}
fn brightness_packet(value: u16) -> [u8; 65] {
    let mut p = packet(1, 2, 0, 0);
    p[5..7].copy_from_slice(&value.to_le_bytes());
    p
}
fn validate_reply(command: u8, reply: &[u8]) -> Result<()> {
    if reply.len() < 7 {
        return Err(
            "마우스 응답 시간 초과 또는 잘린 응답입니다. iCUE를 종료한 후 다시 시도하세요.".into(),
        );
    }
    // Actual wired HARPOON response starts with 0x00, not the outbound address 0x08.
    if reply[0] != 0 || reply[1] != command {
        return Err(
            "다른 명령의 마우스 응답을 받았습니다. iCUE와 동시 제어 중인지 확인하세요.".into(),
        );
    }
    if reply[2] != 0 {
        return Err(format!("마우스가 명령을 거부했습니다 (상태 {}).", reply[2]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_failed_and_mismatched_ack() {
        assert!(validate_reply(2, &[]).is_err());
        assert!(validate_reply(2, &[0, 2, 3, 0, 0, 0, 0]).is_err());
        assert!(validate_reply(2, &[0, 1, 0, 0, 0, 0, 0]).is_err());
        assert!(validate_reply(2, &[0, 2, 0, 0x5e, 0x1b, 0, 0]).is_ok());
    }
    #[test]
    fn power_only_sets_brightness_never_dpi_or_rgb_frames() {
        assert_eq!(&brightness_packet(1000)[..7], &[0, 8, 1, 2, 0, 0xe8, 3]);
        assert!(brightness_packet(0)[5..].iter().all(|v| *v == 0));
    }
    #[test]
    fn dpi_button_is_edge_triggered_and_ignores_other_reports() {
        let mut held = false;
        assert!(!dpi_pressed(&[0, 1, 8], &mut held));
        assert!(!dpi_pressed(&[0, 2, 1], &mut held));
        assert!(dpi_pressed(&[0, 2, 8], &mut held));
        assert!(!dpi_pressed(&[0, 2, 9], &mut held));
        assert!(!dpi_pressed(&[0, 2, 0], &mut held));
        assert!(dpi_pressed(&[0, 2, 8], &mut held));
    }
    #[test]
    fn dpi_cycle_wraps_and_respects_disabled_stages() {
        assert_eq!(next_dpi(0x1f, 4).unwrap(), 0);
        assert_eq!(next_dpi(0b10101, 0).unwrap(), 2);
        assert_eq!(next_dpi(0b00100, 2).unwrap(), 2);
        assert!(next_dpi(0, 0).is_err());
        assert!(next_dpi(0x80, 0).is_err());
    }
    #[test]
    fn dpi_preferences_reject_invalid_ranges_stages_and_unknown_fields() {
        let mut p = DpiProfile {
            values: [400, 1000, 1600, 3000, 5000],
            colors: [[255, 0, 0]; 5],
            mask: 31,
            index: 2,
        };
        assert!(p.validate().is_ok());
        p.values[4] = 10001;
        assert!(p.validate().is_err());
        p.values[4] = 5000;
        p.values[0] = 1700;
        assert!(p.validate().is_err());
        p.values[0] = 400;
        p.mask = 1;
        assert!(p.validate().is_err());
        p.index = 0;
        assert!(p.validate().is_ok());
        p.index = 5;
        assert!(p.validate().is_err());
        let json = r#"{"values":[400,1000,1600,3000,5000],"colors":[[255,0,0],[255,0,0],[255,0,0],[255,0,0],[255,0,0]],"mask":31,"index":0,"keymap":true}"#;
        assert!(serde_json::from_str::<DpiProfile>(json).is_err());
    }
}
