use crate::devices::Result;
use serde::{Deserialize, Serialize};
use std::{
    os::windows::process::CommandExt,
    path::Path,
    process::Command,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub ready: bool,
    pub error: Option<String>,
}
pub type State = Mutex<Status>;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preference {
    enabled: bool,
}

fn task(enabled: bool) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("startup_task.ps1"),
        ])
        .env_remove("PSModulePath")
        .env(
            "RGB_SWITCH_STARTUP_ACTION",
            if enabled { "enable" } else { "disable" },
        )
        .env("RGB_SWITCH_STARTUP_EXE", exe)
        .creation_flags(0x08000000)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("자동 시작 등록 시간이 초과됐습니다. 설정에서 다시 시도하세요.".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let data: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "자동 시작 상태를 확인하지 못했습니다.")?;
    if !output.status.success() || data["enabled"].as_bool() != Some(enabled) {
        return Err(format!(
            "자동 시작 설정 실패. 관리자 권한으로 다시 실행하세요. {}",
            data["error"].as_str().unwrap_or("예약 작업 응답 오류")
        ));
    }
    Ok(())
}
fn preference(path: &Path) -> Result<bool> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Preference>(&bytes)
            .map(|p| p.enabled)
            .map_err(|e| format!("자동 시작 설정 파일 오류: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}
pub fn initialize(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<State>();
        let Ok(mut state) = state.lock() else {
            return;
        };
        let result = app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())
            .and_then(|dir| preference(&dir.join("startup.json")))
            .and_then(|enabled| {
                task(enabled)?;
                Ok(enabled)
            });
        match result {
            Ok(enabled) => state.enabled = enabled,
            Err(error) => state.error = Some(error),
        }
        state.ready = true;
        let _ = app.emit("startup-status", state.clone());
    });
}
#[tauri::command]
pub fn get_startup(state: tauri::State<'_, State>) -> Result<Status> {
    // Startup registration runs in a worker; don't block the UI waiting on its lock.
    Ok(state.try_lock().map(|s| s.clone()).unwrap_or_default())
}
#[tauri::command]
pub async fn set_startup(enabled: bool, app: tauri::AppHandle) -> Result<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        let managed = app.state::<State>();
        let mut state = managed.lock().map_err(|_| "자동 시작 상태 오류")?;
        let path = app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?
            .join("startup.json");
        task(enabled)?;
        let save = (|| -> Result<()> {
            std::fs::create_dir_all(path.parent().ok_or("설정 경로 오류")?)
                .map_err(|e| e.to_string())?;
            let temp = path.with_extension("json.tmp");
            std::fs::write(
                &temp,
                serde_json::to_vec(&Preference { enabled }).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::rename(temp, path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(error) = save {
            let _ = task(state.enabled);
            return Err(error);
        }
        *state = Status {
            enabled,
            ready: true,
            error: None,
        };
        let _ = app.emit("startup-status", state.clone());
        Ok(state.clone())
    })
    .await
    .map_err(|e| e.to_string())?
}
