//! MSI 7E40 / MB800: native lighting effects and global controller switch.
use super::Result;
use hidapi::{HidApi, HidDevice};
use std::{ffi::c_void, marker::PhantomData, rc::Rc};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attributes: *const c_void, initial_owner: i32, name: *const u16)
        -> *mut c_void;
    fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
    fn ReleaseMutex(handle: *mut c_void) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

// The installed MSI software uses this same mutex for its motherboard USB access.
// Ownership is thread-bound; prevent moving this guard to another thread.
struct ControllerLock(*mut c_void, PhantomData<Rc<()>>);
impl ControllerLock {
    fn acquire() -> Result<Self> {
        let name: Vec<u16> = "Global\\Access_USB_Sensors\0".encode_utf16().collect();
        // SAFETY: valid NUL-terminated name, default security, no initial ownership.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "MSI 통신 잠금을 열지 못했습니다: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: live mutex handle; bounded wait, ownership acquired on this thread.
        let status = unsafe { WaitForSingleObject(handle, 1500) };
        if status == 0 || status == 0x80 {
            return Ok(Self(handle, PhantomData));
        }
        unsafe {
            CloseHandle(handle);
        }
        Err("MSI 조명 통신이 사용 중입니다. 잠시 후 다시 시도하세요.".into())
    }
}
impl Drop for ControllerLock {
    fn drop(&mut self) {
        // SAFETY: this thread owns the mutex and the handle is closed once.
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

pub fn board_matches() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("HARDWARE\\DESCRIPTION\\System\\BIOS")
        .and_then(|key| key.get_value::<String, _>("BaseBoardProduct"))
        .is_ok_and(|name| name == "MAG B860M MORTAR WIFI (MS-7E40)")
}

pub struct Board {
    device: HidDevice,
    _lock: ControllerLock,
}
#[derive(Default)]
pub struct PowerState {
    resume: Option<Vec<u8>>,
}
impl Board {
    pub fn open(api: &HidApi) -> Result<Self> {
        if !board_matches() {
            return Err("이 모듈의 대상 메인보드가 아닙니다.".into());
        }
        let lock = ControllerLock::acquire()?;
        let candidates: Vec<_> = api
            .device_list()
            .filter(|d| {
                d.vendor_id() == 0x0db0
                    && d.product_id() == 0x0076
                    && d.interface_number() == 0
                    && d.usage_page() == 0xff00
                    && d.usage() == 1
                    && d.serial_number().is_some_and(|s| s.starts_with("7E40"))
            })
            .collect();
        if candidates.len() != 1 {
            return Err("7E40 MYSTIC LIGHT 컨트롤러를 하나로 식별하지 못했습니다.".into());
        }
        let board = Self {
            device: candidates[0].open_device(api).map_err(|e| e.to_string())?,
            _lock: lock,
        };
        board.lighting_settings()?;
        let mut firmware = [0xcc; 64];
        firmware[0] = 1;
        firmware[1] = 0xb0;
        let response = board.exchange(firmware)?;
        if response[2] != 2 || response[3] != 0 {
            return Err("검증한 MSI 조명 펌웨어 2.0과 다릅니다.".into());
        }
        board.enabled()?;
        Ok(board)
    }
    fn exchange(&self, request: [u8; 64]) -> Result<[u8; 64]> {
        for _ in 0..16 {
            let mut stale = [0; 64];
            if self
                .device
                .read_timeout(&mut stale, 2)
                .map_err(|e| e.to_string())?
                == 0
            {
                break;
            }
        }
        let count = self
            .device
            .write(&request)
            .map_err(|e| format!("메인보드 전송 실패: {e}"))?;
        if count != request.len() {
            return Err("메인보드 명령이 일부만 전송됐습니다.".into());
        }
        let mut response = [0; 64];
        let n = self
            .device
            .read_timeout(&mut response, 750)
            .map_err(|e| e.to_string())?;
        validate_reply(&response[..n])?;
        Ok(response)
    }
    pub fn enabled(&self) -> Result<bool> {
        let r = self.exchange(switch_packet(0xba, false))?;
        decode_switch(r[6])
    }
    pub fn lighting_settings(&self) -> Result<Vec<u8>> {
        let mut report = vec![0; 290];
        report[0] = 0x50;
        let n = self
            .device
            .get_feature_report(&mut report)
            .map_err(|e| e.to_string())?;
        if n != 290 || report[0] != 0x50 {
            return Err("메인보드 조명 설정 응답 형식이 다릅니다.".into());
        }
        Ok(report)
    }
    pub fn inspect_argb_ports(&self) -> Vec<Result<Vec<u8>>> {
        (0..4)
            .map(|port| {
                let mut report = vec![255; 302];
                report[0] = 0x90 + port;
                let n = self
                    .device
                    .get_feature_report(&mut report)
                    .map_err(|e| e.to_string())?;
                if n != 302 || report[0] != 0x90 + port {
                    return Err("ARGB 포트 설정 형식이 다릅니다.".into());
                }
                Ok(report)
            })
            .collect()
    }
    pub fn detect_argb_ports(&self) -> Vec<Result<Vec<u8>>> {
        (0..4)
            .map(|port| {
                let mut request = [0; 64];
                request[0] = 1;
                request[1] = 0x82;
                request[6] = port;
                let reply = self.exchange(request)?;
                if reply[6] != port || reply[8] != 0 {
                    return Err(format!("ARGB 포트 {port} 감지 오류 {}", reply[8]));
                }
                Ok(reply.to_vec())
            })
            .collect()
    }
    pub fn inspect_port_mode(&self) -> Vec<Result<Vec<u8>>> {
        (0..4)
            .map(|port| {
                let mut request = [0; 64];
                request[0] = 1;
                request[1] = 0x80;
                request[6] = port;
                self.exchange(request).map(|r| r.to_vec())
            })
            .collect()
    }
    /// Explicit initialization of ordinary ARGB headers after Gen2 detection.
    /// Captured MSI Scan does 82 detection then 84(port, false) for zero strips.
    /// No mode conversion is attempted if any port reports an identified strip.
    pub fn prepare_legacy_headers(&self) -> Result<()> {
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            for reply in self.detect_argb_ports() {
                let reply = reply?;
                if reply[7] != 0 {
                    return Err("Gen2 조명 장치가 감지되어 일반 ARGB 전환을 중단했습니다.".into());
                }
            }
        }
        for port in 0..4 {
            let reply = self.exchange(legacy_port_packet(port))?;
            if reply[6] != port {
                return Err(format!("ARGB 포트 {port} 모드 전환 응답이 다릅니다."));
            }
        }
        Ok(())
    }
    fn set_switch(&self, enabled: bool) -> Result<()> {
        let response = self.exchange(switch_packet(0xbb, enabled))?;
        if decode_switch(response[6])? != enabled {
            return Err("메인보드가 다른 조명 상태로 응답했습니다.".into());
        }
        if self.enabled()? != enabled {
            return Err("메인보드 조명 상태 재조회가 일치하지 않습니다.".into());
        }
        Ok(())
    }
    /// Raw global switch for diagnostic restoration, not the app's LED power path.
    /// Use set_power to initialize ARGB ports and send native Off to connected fans.
    pub fn power(&self, enabled: bool) -> Result<()> {
        let previous = self.enabled()?;
        if let Err(error) = self.set_switch(enabled) {
            return Err(match self.set_switch(previous) {
                Ok(()) => format!("{error} 이전 스위치 상태로 복원했습니다."),
                Err(restore) => format!("{error} 스위치 복원 실패: {restore}"),
            });
        }
        Ok(())
    }
    pub fn set_power(&self, enabled: bool, state: &mut PowerState) -> Result<()> {
        let previous = self.lighting_settings()?;
        let desired = if enabled {
            on_settings(&previous, state.resume.as_deref())
        } else {
            off_settings(&previous)
        };
        self.apply_native(&desired, &previous)?;
        if enabled {
            state.resume = None;
        } else if has_visible_effect(&previous) {
            // Repeated OFF must not replace the saved effect with an Off packet.
            state.resume = Some(previous);
        }
        Ok(())
    }
    pub fn restore_lighting(&self, previous: &[u8]) -> Result<()> {
        if previous.len() != 290 || previous[0] != 0x50 {
            return Err("메인보드 조명 데이터 형식 오류".into());
        }
        let packet = outgoing_settings(previous);
        self.device
            .send_feature_report(&packet)
            .map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let actual = self.lighting_settings()?;
        if !settings_match(&actual, &packet) {
            let differences: Vec<_> = actual[..289]
                .iter()
                .zip(&packet[..289])
                .enumerate()
                .filter(|(i, (a, b))| {
                    let expected = if [160, 288].contains(i) { 0 } else { **b };
                    **a != expected
                })
                .map(|(i, (a, b))| format!("{i}:{b:02x}>{a:02x}"))
                .collect();
            return Err(format!(
                "메인보드 효과 재조회가 요청과 다릅니다 [{}].",
                differences.join(",")
            ));
        }
        Ok(())
    }
    pub fn lighting(&self, settings: &super::lighting::Lighting) -> Result<()> {
        settings.validate()?;
        let previous = self.lighting_settings()?;
        let desired = if settings.brightness == 0 {
            off_settings(&previous)
        } else {
            effect_packet(&previous, settings)
        };
        self.apply_native(&desired, &previous)
    }
    fn apply_native(&self, desired: &[u8], previous: &[u8]) -> Result<()> {
        let enabled = self.enabled()?;
        self.prepare_legacy_headers()?;
        // Keep the ARGB data output active while sending the native Off effect.
        let result = self
            .set_switch(true)
            .and_then(|_| self.restore_lighting(desired));
        if let Err(e) = result {
            let restore = self.restore_lighting(previous);
            let switch = self.set_switch(enabled);
            return Err(match (restore, switch) {
                (Ok(()), Ok(())) => format!("{e} 이전 메인보드 설정을 복원했습니다."),
                (a, b) => format!("{e} 복원 결과: {a:?}, {b:?}"),
            });
        }
        Ok(())
    }
}
fn off_settings(previous: &[u8]) -> Vec<u8> {
    let mut packet = previous.to_vec();
    for index in [0, 1, 2, 3, 9, 17] {
        let area = &mut packet[1 + index * 16..1 + (index + 1) * 16];
        area[..13].fill(0); // native Off and four black color slots
        area[13] &= !3;
        area[14] = (area[14] & 0xc0) | 0x35; // user color, B100, medium speed
    }
    packet[289] = 0;
    packet
}
fn has_visible_effect(packet: &[u8]) -> bool {
    [0, 1, 2, 3, 9].iter().any(|index| {
        let start = 1 + index * 16;
        let start = if packet[start + 14] & 0x80 != 0 {
            273
        } else {
            start
        };
        let mode = packet[start];
        let option = packet[start + 14];
        mode != 0
            && option & 0x1c != 0
            && !([2, 4].contains(&mode)
                && option & 0x20 != 0
                && packet[start + 1..start + 13].iter().all(|v| *v == 0))
    })
}
fn on_settings(previous: &[u8], resume: Option<&[u8]>) -> Vec<u8> {
    let source = resume.unwrap_or(previous);
    if has_visible_effect(source) {
        source.to_vec()
    } else {
        effect_packet(
            source,
            &super::lighting::Lighting {
                effect: super::lighting::Effect::Static,
                color: [255; 3],
                brightness: 100,
                speed: 2,
            },
        )
    }
}
fn legacy_port_packet(port: u8) -> [u8; 64] {
    let mut packet = [0; 64];
    packet[0] = 1;
    packet[1] = 0x84;
    packet[6] = port;
    packet
}
fn outgoing_settings(previous: &[u8]) -> Vec<u8> {
    let mut packet = previous.to_vec();
    // Captured from the installed MSI MB800 application's volatile Apply:
    // JRGB1 and SelectAll transmit Cycle_Num=255, but GetFeature returns 0
    // for both. Never reuse their readback zeros as the outbound values.
    packet[160] = 255;
    packet[288] = 255;
    packet[289] = 0; // volatile apply, never store in flash
    packet
}
fn settings_match(actual: &[u8], sent: &[u8]) -> bool {
    actual.len() == 290
        && sent.len() == 290
        && actual[..289]
            .iter()
            .enumerate()
            .all(|(i, value)| *value == if [160, 288].contains(&i) { 0 } else { sent[i] })
}
fn effect_packet(previous: &[u8], s: &super::lighting::Lighting) -> Vec<u8> {
    use super::lighting::Effect;
    let mut p = previous.to_vec();
    // Exact MB800 format: 18 lighting areas, 16 bytes each, followed by Store flag.
    // 7E40 FW2.0 exposes JARGB1/2/3, JAF, JRGB1 and SelectAll.
    // The remaining twelve areas are absent and always read as zero.
    for index in [0, 1, 2, 3, 9, 17] {
        let area = &mut p[1 + index * 16..1 + (index + 1) * 16];
        area[0] = match s.effect {
            Effect::Static => 2,
            Effect::Breathe => 4,
            Effect::Spectrum => 5,
        };
        for color in area[1..13].chunks_exact_mut(3) {
            color.copy_from_slice(&s.color);
        }
        area[13] &= !3; // one color, preserving reserved bits
        area[14] = (area[14] & 0xc0)
            | (u8::from(s.effect != Effect::Spectrum) << 5)
            | (s.level(5) << 2)
            | (s.speed - 1);
    }
    p[289] = 0;
    p
}
fn switch_packet(command: u8, enabled: bool) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 1;
    p[1] = command;
    p[6] = u8::from(enabled);
    p
}
fn validate_reply(reply: &[u8]) -> Result<()> {
    if reply.len() != 64 || reply[0] != 1 || reply[1] != 0x5a {
        return Err("메인보드 응답 시간 초과 또는 잘못된 응답입니다.".into());
    }
    Ok(())
}
fn decode_switch(value: u8) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err("메인보드가 알 수 없는 조명 스위치 값을 보냈습니다.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_global_switch_fields_are_written() {
        let p = switch_packet(0xbb, true);
        assert_eq!(&p[..7], &[1, 0xbb, 0, 0, 0, 0, 1]);
        assert!(p[7..].iter().all(|b| *b == 0));
        assert_eq!(switch_packet(0xbb, false)[6], 0);
    }
    #[test]
    fn rejects_invalid_acknowledgements() {
        assert!(validate_reply(&[1, 0x5a]).is_err());
        assert!(validate_reply(&[0; 64]).is_err());
        assert!(decode_switch(2).is_err());
    }
    #[test]
    fn legacy_port_conversion_is_only_the_captured_lighting_command() {
        for port in 0..4 {
            let packet = legacy_port_packet(port);
            assert_eq!(&packet[..8], &[1, 0x84, 0, 0, 0, 0, port, 0]);
            assert!(packet[8..].iter().all(|value| *value == 0));
        }
    }
    #[test]
    fn vendor_write_only_cycle_fields_do_not_hide_other_mismatches() {
        let mut before = vec![0; 290];
        before[0] = 0x50;
        before[16] = 30;
        let sent = outgoing_settings(&before);
        assert_eq!(sent[160], 255);
        assert_eq!(sent[288], 255);
        assert_eq!(sent[16], 30);
        assert_eq!(sent[289], 0);
        assert!(settings_match(&before, &sent));
        for index in [0, 1, 2, 15, 16, 160, 273, 274, 287, 288] {
            let mut wrong = before.clone();
            wrong[index] = wrong[index].wrapping_add(1);
            assert!(!settings_match(&wrong, &sent), "byte {index}");
        }
    }
    #[test]
    fn native_off_can_resume_effects_and_initial_off_falls_back_to_white() {
        use super::super::lighting::{Effect, Lighting};
        let mut initial = vec![0; 290];
        initial[0] = 0x50;
        for i in [0, 1, 2, 3, 9, 17] {
            initial[1 + i * 16 + 14] = 0xb5;
        }
        let breathing = effect_packet(
            &initial,
            &Lighting {
                effect: Effect::Breathe,
                color: [25, 80, 200],
                brightness: 60,
                speed: 3,
            },
        );
        let off = off_settings(&breathing);
        assert!(!has_visible_effect(&off));
        assert_eq!(on_settings(&off, Some(&breathing)), breathing);
        assert_eq!(off_settings(&off), off);
        let white = on_settings(&off, None);
        assert!(has_visible_effect(&white));
        assert_eq!(&white[274..277], &[255, 255, 255]);
        // Selected headers may retain Wave while their shared master is Off.
        initial[1] = 1;
        assert!(!has_visible_effect(&initial));
    }
    #[test]
    fn effects_preserve_absent_headers_and_never_write_flash() {
        use super::super::lighting::{Effect, Lighting};
        let mut before = vec![0xa5; 290];
        before[0] = 0x50;
        let s = Lighting {
            effect: Effect::Static,
            color: [1, 2, 3],
            brightness: 60,
            speed: 2,
        };
        let p = effect_packet(&before, &s);
        for i in 0..18 {
            let offset = 1 + i * 16;
            if [0, 1, 2, 3, 9, 17].contains(&i) {
                assert_eq!(p[offset], 2);
                assert_eq!(&p[offset + 1..offset + 4], &[1, 2, 3]);
                assert_eq!(p[offset + 14] & 0x1c, 3 << 2);
                assert_eq!(p[offset + 15], before[offset + 15]);
            } else {
                assert_eq!(p[offset..offset + 16], before[offset..offset + 16]);
            }
        }
        assert_eq!(p[289], 0);
    }
}
