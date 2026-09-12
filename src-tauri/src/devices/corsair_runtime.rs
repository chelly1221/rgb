//! One owner serializes scans, lighting, and DPI input on the wired mouse.
//! Custom stages are app preferences, never writes to onboard presets or keymaps.
use super::{
    corsair::{next_dpi, DpiListener, DpiProfile, Mouse},
    lighting::{Effect, Lighting},
    Result,
};
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active: bool,
    pub profile: Option<DpiProfile>,
    pub error: Option<String>,
}
struct Active {
    mouse: Mouse,
    listener: DpiListener,
    native: DpiProfile,
    profile: DpiProfile,
    logo: [u8; 3],
    lit: bool,
    checked: Instant,
    beat: Instant,
}
#[derive(Default)]
struct Runtime {
    active: Option<Active>,
    saved: Option<DpiProfile>,
    error: Option<String>,
}
static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();
type Notify = Box<dyn Fn(Status) + Send + Sync>;
static NOTIFY: OnceLock<Notify> = OnceLock::new();
fn runtime() -> &'static Mutex<Runtime> {
    RUNTIME.get_or_init(Mutex::default)
}
fn lock() -> Result<std::sync::MutexGuard<'static, Runtime>> {
    runtime()
        .lock()
        .map_err(|_| "마우스 제어 상태 오류입니다. 앱을 다시 실행하세요.".into())
}
fn api() -> Result<hidapi::HidApi> {
    hidapi::HidApi::new().map_err(|e| e.to_string())
}
pub fn start(notify: impl Fn(Status) + Send + Sync + 'static) {
    let _ = NOTIFY.set(Box::new(notify));
    STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            let update = if let Ok(mut r) = lock() {
                let result = r.active.as_mut().map(Active::tick);
                match result {
                    Some(Ok(true)) => Some(r.status()),
                    Some(Err(error)) => {
                        let restore = r.stop();
                        r.error = Some(format!(
                            "마우스 제어 중단: {error}{}",
                            restore
                                .err()
                                .map(|e| format!(" 복원 실패: {e}"))
                                .unwrap_or_default()
                        ));
                        Some(r.status())
                    }
                    _ => None,
                }
            } else {
                None
            };
            // Never call back into the app while holding the device lock.
            if let (Some(update), Some(notify)) = (update, NOTIFY.get()) {
                notify(update);
            }
            std::thread::sleep(Duration::from_millis(8));
        });
    });
}
pub fn set_saved(profile: DpiProfile) -> Result<()> {
    profile.validate()?;
    lock()?.saved = Some(profile);
    Ok(())
}
pub fn status() -> Result<Status> {
    Ok(lock()?.status())
}
pub fn report_error(error: String) {
    if let Ok(mut r) = lock() {
        r.error = Some(error);
    }
}
pub fn inspect() -> Result<u8> {
    let r = lock()?;
    if let Some(active) = &r.active {
        active.mouse.render_mode()
    } else {
        Mouse::open(&api()?)?.render_mode()
    }
}
pub fn current_dpi() -> Result<u16> {
    let r = lock()?;
    if let Some(active) = &r.active {
        active.mouse.current_dpi()
    } else {
        Mouse::open(&api()?)?.current_dpi()
    }
}
pub fn profile() -> Result<DpiProfile> {
    let r = lock()?;
    if let Some(active) = &r.active {
        Ok(active.profile.clone())
    } else if let Some(saved) = &r.saved {
        Ok(saved.clone())
    } else {
        Mouse::open(&api()?)?.dpi_profile()
    }
}
pub fn lighting(settings: &Lighting) -> Result<()> {
    settings.validate()?;
    if settings.effect != Effect::Static {
        return Err("마우스 로고는 단색을 지원합니다.".into());
    }
    ensure_exclusive()?;
    let mut r = lock()?;
    r.apply(settings)?;
    r.error = None;
    let status = r.status();
    drop(r);
    publish(status);
    Ok(())
}
pub fn configure(profile: DpiProfile, settings: &Lighting) -> Result<DpiProfile> {
    profile.validate()?;
    settings.validate()?;
    if settings.effect != Effect::Static {
        return Err("마우스 조명을 단색으로 설정하세요.".into());
    }
    ensure_exclusive()?;
    let mut r = lock()?;
    r.apply(settings)?;
    let active = r
        .active
        .as_mut()
        .ok_or("마우스 제어를 시작하지 못했습니다.")?;
    let old = active.profile.clone();
    if let Err(error) = active
        .mouse
        .select_dpi(&profile, profile.index)
        .and_then(|_| active.frame_for(&profile))
    {
        let restored = active
            .mouse
            .select_dpi(&old, old.index)
            .and_then(|_| active.frame_for(&old));
        if let Err(restore) = restored {
            let _ = r.stop();
            return Err(format!("{error} 이전 DPI 복원 실패: {restore}"));
        }
        return Err(error);
    }
    active.profile = profile.clone();
    r.saved = Some(profile.clone());
    r.error = None;
    let status = r.status();
    drop(r);
    publish(status);
    Ok(profile)
}
pub fn power(enabled: bool) -> Result<()> {
    ensure_exclusive()?;
    let mut r = lock()?;
    // Preserve custom DPI handling while RGB is off; black includes the indicator.
    if let Some(a) = r.active.as_mut() {
        let previous = a.lit;
        a.lit = enabled;
        if let Err(error) = a.frame_for(&a.profile) {
            a.lit = previous;
            return Err(error);
        }
    } else {
        Mouse::open(&api()?)?.power(enabled)?;
    }
    r.error = None;
    let status = r.status();
    drop(r);
    publish(status);
    Ok(())
}
fn publish(status: Status) {
    if let Some(notify) = NOTIFY.get() {
        notify(status);
    }
}
pub fn shutdown() -> Result<()> {
    lock()?.stop()
}
impl Runtime {
    fn status(&self) -> Status {
        Status {
            active: self.active.is_some(),
            profile: self.active.as_ref().map(|a| a.profile.clone()),
            error: self.error.clone(),
        }
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(a) = self.active.take() {
            let mut native = a.native.clone();
            native.index = if native.mask & (1 << a.profile.index) != 0 {
                a.profile.index
            } else {
                native.index
            };
            let dpi = a.mouse.select_dpi(&native, native.index);
            let mode = a.mouse.finish_software(a.lit);
            dpi.and(mode)
        } else {
            Ok(())
        }
    }
    fn apply(&mut self, settings: &Lighting) -> Result<()> {
        let logo = settings
            .color
            .map(|v| ((u16::from(v) * u16::from(settings.brightness) + 50) / 100) as u8);
        if let Some(a) = &mut self.active {
            let previous = (a.logo, a.lit);
            a.logo = logo;
            a.lit = settings.brightness > 0;
            if let Err(error) = a.frame_for(&a.profile) {
                a.logo = previous.0;
                a.lit = previous.1;
                if a.frame_for(&a.profile).is_err() {
                    let _ = self.stop();
                }
                return Err(error);
            }
            return Ok(());
        }
        let api = api()?;
        let mouse = Mouse::open(&api)?;
        let listener = DpiListener::open(&api)?;
        let native = mouse.dpi_profile()?;
        let profile = self.saved.clone().unwrap_or_else(|| native.clone());
        let result = mouse
            .begin_software_rgb()
            .and_then(|_| mouse.select_dpi(&profile, profile.index));
        if let Err(error) = result {
            let _ = mouse.select_dpi(&native, native.index);
            let _ = mouse.finish_software(true);
            return Err(error);
        }
        self.active = Some(Active {
            mouse,
            listener,
            native,
            profile,
            logo,
            lit: settings.brightness > 0,
            checked: Instant::now(),
            beat: Instant::now(),
        });
        let a = self.active.as_ref().unwrap();
        if let Err(error) = a.frame_for(&a.profile) {
            let _ = self.stop();
            return Err(error);
        }
        Ok(())
    }
}
impl Active {
    fn frame_for(&self, profile: &DpiProfile) -> Result<()> {
        self.mouse.software_frame(
            if self.lit { self.logo } else { [0; 3] },
            if self.lit {
                profile.colors[profile.index as usize]
            } else {
                [0; 3]
            },
        )
    }
    fn tick(&mut self) -> Result<bool> {
        if self.checked.elapsed() >= Duration::from_secs(2) {
            ensure_exclusive()?;
            if self.mouse.render_mode()? != 2 {
                return Err(
                    "다른 앱이 조명 모드를 변경했습니다. 해당 앱을 종료하고 다시 적용하세요."
                        .into(),
                );
            }
            self.checked = Instant::now();
        }
        if self.beat.elapsed() >= Duration::from_secs(10) {
            self.mouse.heartbeat()?;
            self.beat = Instant::now();
        }
        let pressed = self.listener.pressed()?;
        if pressed {
            let next = next_dpi(self.profile.mask, self.profile.index)?;
            self.mouse.select_dpi(&self.profile, next)?;
            self.profile.index = next;
            self.frame_for(&self.profile)?;
        }
        Ok(pressed)
    }
}
fn ensure_exclusive() -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    // Read process names only. Never stop another lighting app or modify its services.
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle == INVALID_HANDLE_VALUE {
            return Err("조명 앱 실행 상태를 확인하지 못했습니다.".into());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(handle, &mut entry);
        let mut icue = false;
        while ok != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length]).eq_ignore_ascii_case("icue.exe")
            {
                icue = true;
                break;
            }
            ok = Process32NextW(handle, &mut entry);
        }
        CloseHandle(handle);
        if icue {
            Err("iCUE가 마우스 조명을 제어하고 있습니다. 트레이의 iCUE를 종료한 뒤 다시 적용하세요.".into())
        } else {
            Ok(())
        }
    }
}
