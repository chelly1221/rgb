use super::Result;
use std::{ffi::c_void, marker::PhantomData, rc::Rc};
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(a: *const c_void, owner: i32, name: *const u16) -> *mut c_void;
    fn WaitForSingleObject(h: *mut c_void, ms: u32) -> u32;
    fn ReleaseMutex(h: *mut c_void) -> i32;
    fn CloseHandle(h: *mut c_void) -> i32;
}
pub struct NamedLock(*mut c_void, PhantomData<Rc<()>>);
impl NamedLock {
    pub fn acquire(name: &str) -> Result<Self> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated owned UTF-16 name and standard mutex arguments.
        let h = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if h.is_null() {
            return Err(format!(
                "장치 통신 잠금을 열지 못했습니다: {}",
                std::io::Error::last_os_error()
            ));
        }
        let status = unsafe { WaitForSingleObject(h, 1500) };
        if status == 0 || status == 0x80 {
            Ok(Self(h, PhantomData))
        } else {
            unsafe {
                CloseHandle(h);
            }
            Err("다른 프로그램이 장치 통신을 사용 중입니다. 잠시 후 다시 시도하세요.".into())
        }
    }
}
impl Drop for NamedLock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}
