//! Exact-match Palit RTX 3070 adapter using the installed NVIDIA driver's NVAPI.
//! Register facts and interface IDs are documented in docs/PROTOCOLS.md.
use super::Result;
use libloading::Library;
use std::{ffi::c_void, mem, ptr};

type Handle = *mut c_void;
type Query = unsafe extern "C" fn(u32) -> *mut c_void;
type Init = unsafe extern "C" fn() -> i32;
type Enumerate = unsafe extern "C" fn(*mut Handle, *mut i32) -> i32;
type Pci = unsafe extern "C" fn(Handle, *mut u32, *mut u32, *mut u32, *mut u32) -> i32;
type Transfer = unsafe extern "C" fn(Handle, *mut I2cInfo, *mut u32) -> i32;

#[repr(C)]
struct I2cInfo {
    version: u32,
    display_mask: u32,
    ddc: u8,
    address: u8,
    register: *mut u8,
    register_len: u32,
    data: *mut u8,
    data_len: u32,
    speed: u32,
    speed_khz: u32,
    port: u8,
    port_set: u32,
}

pub struct Gpu {
    _dll: Library,
    handle: Handle,
    read: Transfer,
    write: Transfer,
    unload: Init,
}

fn status(value: i32) -> Result<()> {
    if value == 0 {
        Ok(())
    } else {
        Err(format!("NVIDIA 드라이버 통신 오류 ({value})"))
    }
}

unsafe fn function<T: Copy>(query: Query, id: u32) -> Result<T> {
    let address = query(id);
    if address.is_null() || mem::size_of::<T>() != mem::size_of::<*mut c_void>() {
        return Err(format!(
            "NVIDIA 드라이버가 필요한 인터페이스 {id:08X}를 제공하지 않습니다."
        ));
    }
    // Call sites supply the matching NVAPI ABI type; the library outlives all pointers.
    Ok(mem::transmute_copy(&address))
}

impl Gpu {
    pub fn open() -> Result<Self> {
        let system = std::env::var_os("SystemRoot").ok_or("Windows 경로를 찾지 못했습니다.")?;
        let path = std::path::PathBuf::from(system).join("System32/nvapi64.dll");
        unsafe {
            let dll = Library::new(path)
                .map_err(|e| format!("NVIDIA 드라이버를 불러오지 못했습니다: {e}"))?;
            let query = *dll
                .get::<Query>(b"nvapi_QueryInterface\0")
                .map_err(|e| e.to_string())?;
            let init: Init = function(query, 0x0150e828)?;
            let unload: Init = function(query, 0xd22bdd7e)?;
            let enumerate: Enumerate = function(query, 0xe5ac921f)?;
            let pci: Pci = function(query, 0x2ddfb66e)?;
            let read: Transfer = function(query, 0x4d7b0709)?;
            let write: Transfer = function(query, 0x283ac65a)?;
            status(init())?;
            let mut gpu = Self {
                _dll: dll,
                handle: ptr::null_mut(),
                read,
                write,
                unload,
            };
            let mut handles = [ptr::null_mut(); 64];
            let mut count = 0;
            status(enumerate(handles.as_mut_ptr(), &mut count))?;
            if !(0..=64).contains(&count) {
                return Err("잘못된 GPU 목록입니다.".into());
            }
            let mut found = Vec::new();
            for handle in &handles[..count as usize] {
                let (mut dev, mut sub, mut revision, mut ext) = (0, 0, 0, 0);
                status(pci(*handle, &mut dev, &mut sub, &mut revision, &mut ext))?;
                if matches_pc(dev, sub) {
                    found.push(*handle);
                }
            }
            if found.len() != 1 {
                return Err(
                    "이 PC용 RTX 3070(10DE:2484 / 1569:2484)을 하나로 식별하지 못했습니다.".into(),
                );
            }
            gpu.handle = found[0];
            if gpu.read_register(0)? != 0 || gpu.read_register(0xe0)? >= 5 {
                return Err("RTX 3070의 RGB 컨트롤러 응답이 검증된 형식과 다릅니다.".into());
            }
            Ok(gpu)
        }
    }

    fn transfer(&self, register: u8, value: &mut u8, write: bool) -> Result<()> {
        let mut reg = register;
        let mut info = I2cInfo {
            version: (mem::size_of::<I2cInfo>() as u32) | (3 << 16),
            display_mask: 0,
            ddc: 0,
            address: 0x49 << 1,
            register: &mut reg,
            register_len: 1,
            data: value,
            data_len: 1,
            speed: 0xffff,
            speed_khz: 0,
            port: 1,
            port_set: 1,
        };
        let mut extra = 0;
        // All pointers refer to live stack allocations for this synchronous driver call.
        unsafe {
            status((if write { self.write } else { self.read })(
                self.handle,
                &mut info,
                &mut extra,
            ))
        }
    }

    pub fn read_register(&self, register: u8) -> Result<u8> {
        let mut value = 0;
        self.transfer(register, &mut value, false)?;
        Ok(value)
    }

    pub fn snapshot(&self) -> Result<Vec<u8>> {
        REGISTERS.iter().map(|r| self.read_register(*r)).collect()
    }

    pub fn apply(&self, values: &[u8]) -> Result<()> {
        if values.len() != REGISTERS.len() {
            return Err("GPU 복원 데이터의 길이가 잘못되었습니다.".into());
        }
        // Color/brightness first; mode and control last. Never touch fan/clock/voltage registers.
        for (register, value) in REGISTERS.iter().zip(values) {
            let mut byte = *value;
            self.transfer(*register, &mut byte, true)?;
        }
        if self.snapshot()? != values {
            return Err("GPU 조명 설정을 다시 읽은 값이 일치하지 않습니다.".into());
        }
        Ok(())
    }

    pub fn power(&self, enabled: bool, saved: Option<&[u8]>) -> Result<()> {
        if enabled {
            if let Some(values) = saved {
                self.apply(values)
            } else {
                self.apply(&[255, 255, 255, 255, 0, 1])
            }
        } else {
            let mut values = self.snapshot()?;
            values[4] = 0;
            values[5] = 0;
            self.apply(&values)
        }
    }
    pub fn effect_snapshot(&self) -> Result<Vec<u8>> {
        EFFECT_REGISTERS
            .iter()
            .map(|r| self.read_register(*r))
            .collect()
    }
    pub fn restore_effect(&self, values: &[u8]) -> Result<()> {
        if values.len() != EFFECT_REGISTERS.len() {
            return Err("GPU 효과 데이터 길이 오류".into());
        }
        for (register, value) in EFFECT_REGISTERS.iter().zip(values) {
            let mut byte = *value;
            self.transfer(*register, &mut byte, true)?;
        }
        if self.effect_snapshot()? != values {
            return Err("GPU 효과 재조회가 요청과 다릅니다.".into());
        }
        Ok(())
    }
    pub fn lighting(&self, s: &super::lighting::Lighting) -> Result<()> {
        use super::lighting::Effect;
        s.validate()?;
        let previous = self.effect_snapshot()?;
        let mut next = previous.clone();
        next[..3].copy_from_slice(&s.color);
        next[3] = s.level(255);
        next[4..7].fill(0); // secondary color for the breathing trough
        let period: u16 = match s.speed {
            1 => 2000,
            2 => 1000,
            _ => 500,
        };
        let bytes = period.to_le_bytes();
        next[7..9].copy_from_slice(&bytes);
        next[9..11].copy_from_slice(&bytes);
        next[11] = 0;
        next[12] = match s.speed {
            1 => 80,
            2 => 40,
            _ => 20,
        };
        next[13] = 0x1f;
        next[14] = u8::from(s.effect == Effect::Spectrum);
        next[15] = if s.brightness == 0 {
            0
        } else if s.effect == Effect::Breathe {
            0x22
        } else {
            1
        };
        if let Err(e) = self.restore_effect(&next) {
            return Err(match self.restore_effect(&previous) {
                Ok(()) => format!("{e} 이전 GPU 효과로 복원했습니다."),
                Err(r) => format!("{e} 복원 실패: {r}"),
            });
        }
        Ok(())
    }
}
const EFFECT_REGISTERS: [u8; 16] = [
    0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x62, 0x63, 0x64, 0x65, 0xe1, 0xe2, 0xe3, 0xe0, 0x60,
];

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            (self.unload)();
        }
    }
}
const REGISTERS: [u8; 6] = [0x6c, 0x6d, 0x6e, 0x6f, 0xe0, 0x60];
fn matches_pc(device: u32, subsystem: u32) -> bool {
    device == 0x248410de && subsystem == 0x24841569
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_hardware_gate() {
        assert!(matches_pc(0x248410de, 0x24841569));
        assert!(!matches_pc(0x248810de, 0x24841569));
        assert!(!matches_pc(0x248410de, 0x24841462));
    }
    #[test]
    fn ffi_layout_matches_x64_nvapi() {
        assert_eq!(mem::size_of::<I2cInfo>(), 64);
    }
}
