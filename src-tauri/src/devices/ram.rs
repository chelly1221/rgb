//! This PC's two CMH128GX5M2B6400C42 modules, via the embedded signed Corsair driver.
//! Only lighting-controller registers at 19/1B are accessible through this module.
use super::{driver_bundle::Bundle, lock::NamedLock, Result};
use libloading::Library;
use std::{
    ffi::c_void,
    os::windows::process::CommandExt,
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};
const ADDRESSES: [u8; 2] = [0x19, 0x1b];
type Open = unsafe extern "system" fn() -> u32;
type Close = unsafe extern "system" fn() -> i32;
type Read = unsafe extern "system" fn(i32, *mut u8) -> i32;
type Write = unsafe extern "system" fn(i32, u8) -> i32;

fn verify_pc() -> Result<()> {
    // Fixed script, no frontend arguments. Validate the OS-assigned I/O range before using it.
    let script = include_str!("ram_verify.ps1");
    let output = Command::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env_remove("PSModulePath")
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    verification_result(
        output.status.success(),
        &String::from_utf8_lossy(&output.stdout),
    )
}

fn verification_result(success: bool, output: &str) -> Result<()> {
    if success && output.trim() == "verified" {
        return Ok(());
    }
    Err(match output.trim() {
        "failed:cim" => "Windows 장치 조회 모듈을 불러오지 못했습니다. Windows PowerShell 구성을 확인하세요.",
        "failed:board" => "메모리 제어 대상으로 검증한 MSI MS-7E40 메인보드를 확인하지 못했습니다.",
        "failed:memory" => "검증한 CORSAIR CMH128GX5M2B6400C42 메모리 두 개를 확인하지 못했습니다.",
        "failed:smbus" => "메모리 SMBus 장치 또는 I/O 주소를 확인하지 못했습니다. 관리자 권한으로 다시 실행하세요.",
        _ => "메모리 구성 진단을 실행하지 못했습니다. Windows PowerShell 실행 상태를 확인하세요.",
    }.into())
}
pub struct Ram {
    _library: Library,
    _bundle: Bundle,
    close: Close,
    read: Read,
    write: Write,
    _locks: Vec<NamedLock>,
}
impl Drop for Ram {
    fn drop(&mut self) {
        unsafe {
            (self.close)();
        }
    }
}
impl Ram {
    pub fn open() -> Result<Self> {
        super::driver_bundle::require_admin()?;
        verify_pc()?;
        let bundle = Bundle::prepare()?;
        // SAFETY: protected absolute path, embedded and extracted hashes checked; files held read-only.
        let library = unsafe { Library::new(&bundle.dll) }.map_err(|e| e.to_string())?;
        let mut table = [std::ptr::null_mut::<c_void>(); 128];
        // Inspected export copies 61 pointers. Buffer is deliberately larger than its fixed ABI.
        unsafe {
            library
                .get::<unsafe extern "system" fn(*mut *mut c_void)>(b"CrGetLLAccessInterface\0")
                .map_err(|e| e.to_string())?(table.as_mut_ptr());
        }
        if [3, 4, 11, 12].iter().any(|i| table[*i].is_null()) {
            return Err("Corsair 드라이버 함수가 없습니다.".into());
        }
        let open: Open = unsafe { std::mem::transmute(table[3]) };
        let close: Close = unsafe { std::mem::transmute(table[4]) };
        let read: Read = unsafe { std::mem::transmute(table[12]) };
        let write: Write = unsafe { std::mem::transmute(table[11]) };
        bundle.start_service(open as usize)?;
        let status = unsafe { open() };
        if status != 0 {
            return Err(if status == 5 {
                "메모리 제어에는 관리자 권한이 필요합니다. RGB Switch를 관리자 권한으로 실행하세요."
                    .into()
            } else {
                format!("Corsair 메모리 드라이버 열기 실패: {status}")
            });
        }
        let mut ram = Self {
            _library: library,
            _bundle: bundle,
            close,
            read,
            write,
            _locks: Vec::new(),
        };
        for name in [r"Global\Access_SMBUS", r"Global\Access_SMBUS.HTP.Method"] {
            ram._locks.push(NamedLock::acquire(name)?);
        }
        for address in ADDRESSES {
            if ram.byte(address, 0x43, None)? != 0x1a || ram.byte(address, 0x44, None)? != 4 {
                return Err("검증한 Corsair DDR5 조명 컨트롤러와 다릅니다.".into());
            }
            let info = ram.buffer(address, 0x61, 0, 32)?;
            if info[..4] != [0x1c, 0x1b, 1, 9] || info[28] != 4 || info[8..12] != [5, 0, 6, 0] {
                return Err("검증한 Corsair RGB 메모리 펌웨어와 다릅니다.".into());
            }
        }
        Ok(ram)
    }
    fn rd(&self, offset: u8) -> Result<u8> {
        if ![0, 5].contains(&offset) {
            return Err("허용하지 않은 SMBus 레지스터입니다.".into());
        }
        let mut v = 0;
        if unsafe { (self.read)(0x4000 + i32::from(offset), &mut v) } == 0 {
            return Err("SMBus 읽기 실패".into());
        }
        Ok(v)
    }
    fn wr(&self, offset: u8, value: u8) -> Result<()> {
        if ![0, 2, 3, 4, 5].contains(&offset) {
            return Err("허용하지 않은 SMBus 레지스터입니다.".into());
        }
        if unsafe { (self.write)(0x4000 + i32::from(offset), value) } == 0 {
            return Err("SMBus 전송 실패".into());
        }
        Ok(())
    }
    fn byte(&self, address: u8, reg: u8, value: Option<u8>) -> Result<u8> {
        validate_register(address, reg, value)?;
        if self.rd(0)? & 1 != 0 {
            return Err("SMBus가 사용 중입니다. 버스를 초기화하지 않고 중단했습니다.".into());
        }
        self.wr(0, 0x9e)?;
        self.wr(4, (address << 1) | u8::from(value.is_none()))?;
        self.wr(3, reg)?;
        if let Some(v) = value {
            self.wr(5, v)?;
        }
        self.wr(2, 0x48)?;
        let deadline = Instant::now() + Duration::from_millis(100);
        while Instant::now() < deadline {
            let status = self.rd(0)?;
            if status & 1 == 0 && status & 0x1e != 0 {
                let read = if status & 0x1c == 0 && status & 2 != 0 {
                    self.rd(5)
                } else {
                    Err(format!("메모리 {address:02X} 레지스터 {reg:02X} {value:?} 응답 실패 ({status:02X})"))
                };
                self.wr(0, status & 0x9e)?;
                return read;
            }
            sleep(Duration::from_millis(1));
        }
        Err("SMBus 응답 시간 초과. 강제 초기화 없이 중단했습니다.".into())
    }
    fn buffer(&self, address: u8, command: u8, id: u8, len: usize) -> Result<Vec<u8>> {
        self.byte(address, command, Some(id))?;
        self.byte(address, 0x21, Some(0))?;
        let data = (0..len)
            .map(|_| self.byte(address, 0x40, None))
            .collect::<Result<Vec<_>>>()?;
        if self.byte(address, 0x42, None)? != crc8(&data) {
            return Err("메모리 응답 CRC가 다릅니다.".into());
        }
        Ok(data)
    }
    pub fn snapshot(&self) -> Result<[Vec<u8>; 2]> {
        Ok([
            self.buffer(ADDRESSES[0], 0x63, 1, 20)?,
            self.buffer(ADDRESSES[1], 0x63, 1, 20)?,
        ])
    }
    fn apply_one(&self, address: u8, effect: &[u8]) -> Result<()> {
        if effect.len() != 20 {
            return Err("메모리 효과 길이 오류".into());
        }
        self.byte(address, 0x0b, Some(0))?;
        self.byte(address, 0x21, Some(0))?;
        for v in effect {
            self.byte(address, 0x20, Some(*v))?;
        }
        if self.byte(address, 0x42, None)? != crc8(effect) {
            return Err("메모리 전송 CRC 불일치: 적용하지 않았습니다.".into());
        }
        self.byte(address, 0x82, Some(1))?;
        for _ in 0..10 {
            sleep(Duration::from_millis(10));
            // This firmware temporarily NACKs its address while applying an effect.
            // A failed status read is never success; retry only this read within the bound.
            if self
                .byte(address, 0x30, None)
                .is_ok_and(|status| status & 8 == 0)
            {
                if self.buffer(address, 0x63, 1, 20)? == effect {
                    return Ok(());
                }
                return Err("메모리 효과 재조회가 일치하지 않습니다.".into());
            }
        }
        Err("메모리 효과 적용 시간 초과".into())
    }
    pub fn restore(&self, previous: &[Vec<u8>; 2]) -> Result<()> {
        let mut errors = Vec::new();
        for (address, effect) in ADDRESSES.iter().zip(previous) {
            if let Err(e) = self.apply_one(*address, effect) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" / "))
        }
    }
    pub fn power(&self, enabled: bool) -> Result<()> {
        let previous = self.snapshot()?;
        for (address, effect) in ADDRESSES.iter().zip(&previous) {
            let mut desired = effect.clone();
            desired[7] = if enabled { 255 } else { 0 };
            desired[11] = desired[7];
            if let Err(e) = self.apply_one(*address, &desired) {
                return Err(match self.restore(&previous) {
                    Ok(()) => format!("{e} 이전 메모리 효과로 복원했습니다."),
                    Err(r) => format!("{e} 복원 실패: {r}"),
                });
            }
        }
        Ok(())
    }
    pub fn lighting(&self, settings: &super::lighting::Lighting) -> Result<()> {
        settings.validate()?;
        let previous = self.snapshot()?;
        let desired = effect_data(settings);
        for address in ADDRESSES {
            if let Err(e) = self.apply_one(address, &desired) {
                return Err(match self.restore(&previous) {
                    Ok(()) => format!("{e} 이전 메모리 효과로 복원했습니다."),
                    Err(r) => format!("{e} 복원 실패: {r}"),
                });
            }
        }
        Ok(())
    }
}
fn effect_data(s: &super::lighting::Lighting) -> [u8; 20] {
    use super::lighting::Effect;
    let mut data = [0; 20];
    data[0] = match s.effect {
        Effect::Static => 0x10,
        Effect::Breathe => 1,
        Effect::Spectrum => 8,
    };
    data[1] = s.speed - 1;
    data[2] = u8::from(s.effect != Effect::Spectrum);
    data[4..7].copy_from_slice(&s.color);
    data[8..11].copy_from_slice(&s.color);
    data[7] = s.level(255);
    data[11] = data[7];
    data
}
fn validate_register(address: u8, reg: u8, value: Option<u8>) -> Result<()> {
    let allowed = ADDRESSES.contains(&address)
        && match value {
            None => [0x43, 0x44, 0x40, 0x42, 0x30].contains(&reg),
            Some(v) => match reg {
                0x61 | 0x21 | 0x0b => v == 0,
                0x63 | 0x82 => v == 1,
                0x20 => true,
                _ => false,
            },
        };
    if allowed {
        Ok(())
    } else {
        Err("허용하지 않은 메모리 조명 요청입니다.".into())
    }
}
fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for b in data {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 7
            } else {
                crc << 1
            };
        }
    }
    crc
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verification_requires_success_and_distinguishes_failure_stages() {
        assert!(verification_result(true, "verified\r\n").is_ok());
        assert!(verification_result(false, "verified").is_err());
        assert!(verification_result(true, "").is_err());
        assert!(verification_result(false, "failed:smbus")
            .unwrap_err()
            .contains("SMBus"));
    }
    #[test]
    fn crc_known_check_value() {
        assert_eq!(crc8(b"123456789"), 0xf4);
    }
    #[test]
    fn static_and_animation_effects_have_bounded_brightness_and_no_reserved_writes() {
        use super::super::lighting::{Effect, Lighting};
        for (effect, mode) in [
            (Effect::Static, 0x10),
            (Effect::Breathe, 1),
            (Effect::Spectrum, 8),
        ] {
            let s = Lighting {
                effect,
                color: [255, 90, 140],
                brightness: 60,
                speed: 3,
            };
            let p = effect_data(&s);
            assert_eq!(p[0], mode);
            assert_eq!(p[1], 2);
            assert_eq!((p[7], p[11]), (153, 153));
            assert_eq!(&p[4..7], &s.color);
            assert!(p[12..].iter().all(|v| *v == 0));
        }
    }
    #[test]
    fn rejects_spd_firmware_and_other_addresses() {
        assert!(validate_register(0x19, 0x43, None).is_ok());
        for address in [0x50, 0x51, 0x58, 0x18] {
            assert!(validate_register(address, 0x20, Some(1)).is_err());
        }
        assert!(validate_register(0x19, 0x23, Some(0)).is_err());
        assert!(validate_register(0x19, 0x82, Some(2)).is_err());
    }
}
