//! Personal-build Corsair transport, embedded unchanged and pinned to inspected hashes.
use super::{lock::NamedLock, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST},
    System::Services::*,
    UI::Shell::IsUserAnAdmin,
};

const DLL: &[u8] = include_bytes!("../../private-driver/CorsairLLAccessLib64.dll");
const SYS: &[u8] = include_bytes!("../../private-driver/CorsairLLAccess64.sys");
const EULA: &[u8] = include_bytes!("../../private-driver/EULA.html");
const DLL_HASH: &str = "de834e260b50bcb00063618004ee664dc7117bea90e03c1fa04c2f619daac29e";
const SYS_HASH: &str = "020acbf028c67c0936798ad7ba8436d0d58603912f5019a5e56898c2211cf056";
const DIRECTORY: &str = r"C:\Program Files\RGB Switch\drivers\corsair-de834e26-020acbf0";

pub struct Bundle {
    pub dll: PathBuf,
    sys: PathBuf,
    _files: Vec<File>,
    _lock: NamedLock,
}
pub fn require_admin() -> Result<()> {
    if unsafe { IsUserAnAdmin() } == 0 {
        return Err(
            "메모리 제어에는 관리자 권한이 필요합니다. RGB Switch를 관리자 권한으로 실행하세요."
                .into(),
        );
    }
    Ok(())
}
fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn materialize(path: &Path, bytes: &[u8], expected: &str) -> Result<File> {
    if hash(bytes) != expected {
        return Err("내장 드라이버의 검증값이 다릅니다. 올바른 파일로 다시 빌드하세요.".into());
    }
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| format!("내장 드라이버 저장 실패: {e}"))?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("내장 드라이버 준비 실패: {e}")),
    }
    // Deny write/delete sharing for the lifetime of the loaded transport.
    let file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .map_err(|e| e.to_string())?;
    if hash(&fs::read(path).map_err(|e| e.to_string())?) != expected {
        return Err("앱 전용 드라이버 파일이 검증한 파일과 다릅니다. 로드를 중단했습니다.".into());
    }
    Ok(file)
}
impl Bundle {
    pub fn prepare() -> Result<Self> {
        require_admin()?;
        let lock = NamedLock::acquire(r"Global\RGBSwitch.MemoryDriver")?;
        let directory = Path::new(DIRECTORY);
        fs::create_dir_all(directory)
            .map_err(|e| format!("앱 전용 드라이버 폴더 생성 실패: {e}"))?;
        let dll = directory.join("CorsairLLAccessLib64.dll");
        let sys = directory.join("CorsairLLAccess64.sys");
        let files = vec![
            materialize(&dll, DLL, DLL_HASH)?,
            materialize(&sys, SYS, SYS_HASH)?,
        ];
        let license = directory.join("EULA.html");
        if !license.exists() {
            fs::write(license, EULA).map_err(|e| e.to_string())?;
        }
        Ok(Self {
            dll,
            sys,
            _files: files,
            _lock: lock,
        })
    }
    /// The pinned DLL derives its device name from its own location. Read only its
    /// inspected, initialized service-name buffer; never call its installer/uninstaller.
    pub fn start_service(&self, open_address: usize) -> Result<()> {
        let base = open_address
            .checked_sub(0xba80)
            .ok_or("드라이버 ABI 주소 오류")?;
        // SAFETY: caller loaded the exact SHA-pinned DLL and obtained table[3] (RVA BA80).
        // The initialized service-name buffer at RVA 4C860 contains 260 UTF-16 units.
        let name_data = unsafe { std::slice::from_raw_parts((base + 0x4c860) as *const u16, 260) };
        let len = name_data
            .iter()
            .position(|c| *c == 0)
            .ok_or("드라이버 서비스 이름 오류")?;
        let name = String::from_utf16(&name_data[..len]).map_err(|e| e.to_string())?;
        if !valid_service_name(&name) {
            return Err("내장 드라이버 서비스 이름이 예상 형식과 다릅니다.".into());
        }
        let scm = ServiceHandle(unsafe {
            OpenSCManagerW(
                std::ptr::null(),
                std::ptr::null(),
                SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE,
            )
        });
        if scm.0.is_null() {
            return Err(format!(
                "드라이버 등록 권한 확인 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
        let wide_name = wide(&name);
        let access = SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS | SERVICE_START;
        let mut service = ServiceHandle(unsafe { OpenServiceW(scm.0, wide_name.as_ptr(), access) });
        if service.0.is_null() {
            if std::io::Error::last_os_error().raw_os_error()
                != Some(ERROR_SERVICE_DOES_NOT_EXIST as i32)
            {
                return Err(format!(
                    "드라이버 서비스 조회 실패: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let path = wide(self.sys.to_str().ok_or("드라이버 경로 오류")?);
            let display = wide("RGB Switch Memory Access (Corsair)");
            service = ServiceHandle(unsafe {
                CreateServiceW(
                    scm.0,
                    wide_name.as_ptr(),
                    display.as_ptr(),
                    access,
                    SERVICE_KERNEL_DRIVER,
                    SERVICE_DEMAND_START,
                    SERVICE_ERROR_NORMAL,
                    path.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            });
            if service.0.is_null() {
                return Err(format!(
                    "내장 드라이버 등록 실패: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        // Do not start or modify a colliding service that points at another binary.
        let mut config = [0u64; 1024];
        let mut needed = 0;
        if unsafe {
            QueryServiceConfigW(
                service.0,
                config.as_mut_ptr().cast(),
                std::mem::size_of_val(&config) as u32,
                &mut needed,
            )
        } == 0
        {
            return Err(format!(
                "드라이버 서비스 구성 조회 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
        let cfg = unsafe { &*config.as_ptr().cast::<QUERY_SERVICE_CONFIGW>() };
        // QueryServiceConfigW returns NUL-terminated strings inside this owned buffer.
        let path = cfg.lpBinaryPathName;
        let begin = config.as_ptr() as usize;
        let end = begin + std::mem::size_of_val(&config);
        if path.is_null() || (path as usize) < begin || (path as usize) >= end {
            return Err("드라이버 서비스 경로 응답 오류".into());
        }
        let units = unsafe { std::slice::from_raw_parts(path, (end - path as usize) / 2) };
        let n = units
            .iter()
            .position(|x| *x == 0)
            .ok_or("드라이버 서비스 경로 응답 오류")?;
        let actual = String::from_utf16_lossy(&units[..n]);
        if !service_path_matches(&actual, self.sys.to_str().unwrap())
            || cfg.dwServiceType != SERVICE_KERNEL_DRIVER
            || cfg.dwStartType != SERVICE_DEMAND_START
        {
            return Err("앱 전용 드라이버 서비스의 경로 또는 시작 방식이 다릅니다. 기존 서비스를 변경하지 않고 중단했습니다.".into());
        }
        if unsafe { StartServiceW(service.0, 0, std::ptr::null()) } == 0
            && std::io::Error::last_os_error().raw_os_error()
                != Some(ERROR_SERVICE_ALREADY_RUNNING as i32)
        {
            return Err(format!(
                "내장 드라이버 시작 실패: {}. Windows에서 차단한 드라이버는 로드하지 않습니다.",
                std::io::Error::last_os_error()
            ));
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut status: SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
            let mut size = 0;
            if unsafe {
                QueryServiceStatusEx(
                    service.0,
                    SC_STATUS_PROCESS_INFO,
                    (&mut status as *mut SERVICE_STATUS_PROCESS).cast(),
                    std::mem::size_of_val(&status) as u32,
                    &mut size,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if status.dwCurrentState == SERVICE_RUNNING {
                return Ok(());
            }
            if status.dwCurrentState == SERVICE_STOPPED || Instant::now() >= deadline {
                return Err(format!(
                    "내장 드라이버 시작 미완료 (Windows 오류 {}).",
                    status.dwWin32ExitCode
                ));
            }
            sleep(Duration::from_millis(50));
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn service_path_matches(actual: &str, expected: &str) -> bool {
    // SCM normalizes kernel driver paths to the NT DOS-device namespace.
    actual
        .strip_prefix(r"\??\")
        .unwrap_or(actual)
        .eq_ignore_ascii_case(expected)
}
fn valid_service_name(s: &str) -> bool {
    s.strip_prefix("CorsairLLAccess")
        .is_some_and(|v| v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}
struct ServiceHandle(SC_HANDLE);
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseServiceHandle(self.0);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_files_are_the_inspected_versions() {
        assert_eq!(hash(DLL), DLL_HASH);
        assert_eq!(hash(SYS), SYS_HASH);
    }
    #[test]
    fn only_the_exact_driver_path_is_accepted() {
        assert!(service_path_matches(
            r"\??\C:\Program Files\RGB Switch\driver.sys",
            r"C:\Program Files\RGB Switch\driver.sys"
        ));
        assert!(!service_path_matches(
            r"\??\C:\other\driver.sys",
            r"C:\Program Files\RGB Switch\driver.sys"
        ));
        assert!(!service_path_matches(
            r"C:\Program Files\RGB Switch\driver.sys extra",
            r"C:\Program Files\RGB Switch\driver.sys"
        ));
    }
    #[test]
    fn service_names_cannot_redirect_to_other_services() {
        assert!(valid_service_name(
            "CorsairLLAccess8F050F5E415C1A5882EB9FF7CE2BC59B7BE3A953"
        ));
        for s in ["CorsairService", "Spooler", "CorsairLLAccess/../../foo"] {
            assert!(!valid_service_name(s));
        }
    }
}
