mod desktop;
pub mod devices;
mod startup;

use devices::corsair::DpiProfile;
use devices::corsair_runtime;
use devices::lighting::Lighting;
use devices::{Device, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};

#[derive(Default)]
struct Session {
    last_requested: HashMap<String, bool>,
    lighting: HashMap<String, Lighting>,
    motherboard_power: devices::msi::PowerState,
}
type AppState = Arc<Mutex<Session>>;

#[tauri::command]
async fn scan_devices(state: tauri::State<'_, AppState>) -> Result<Vec<Device>> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = state
            .lock()
            .map_err(|_| "앱 상태 오류입니다. 다시 실행하세요.")?;
        let mut devices = devices::scan()?;
        for d in &mut devices {
            if d.supported {
                d.last_requested = session.last_requested.get(&d.key).copied();
                d.last_lighting = session.lighting.get(&d.key).cloned();
            }
            if d.id == 0 {
                if let Some(error) = corsair_runtime::status()?.error {
                    d.last_requested = None;
                    d.detail = error;
                }
            }
        }
        Ok(devices)
    })
    .await
    .map_err(|e| format!("장치 조회 작업 실패: {e}"))?
}

#[derive(Deserialize)]
struct Target {
    id: u32,
    key: String,
}
#[derive(Serialize)]
struct Outcome {
    id: u32,
    error: Option<String>,
}

#[tauri::command]
async fn set_power(
    targets: Vec<Target>,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Outcome>> {
    if targets.is_empty() || targets.len() > 5 {
        return Err("제어할 장치를 1~5개 선택하세요.".into());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = state
            .lock()
            .map_err(|_| "앱 상태 오류입니다. 다시 실행하세요.")?;
        let mut seen = std::collections::HashSet::new();
        let mut outcomes = Vec::new();
        for target in targets {
            let result = if seen.insert(target.id) {
                if enabled {
                    if let Some(settings) = session.lighting.get(&target.key) {
                        let mut on = settings.clone();
                        if on.brightness == 0 {
                            on.brightness = 100;
                        }
                        devices::lighting::apply(target.id, &target.key, &on)
                    } else {
                        devices::power(target.id, &target.key, true, &mut session.motherboard_power)
                    }
                } else {
                    devices::power(
                        target.id,
                        &target.key,
                        false,
                        &mut session.motherboard_power,
                    )
                }
            } else {
                Err("중복된 장치 요청입니다.".into())
            };
            if result.is_ok() {
                session.last_requested.insert(target.key, enabled);
            } else {
                session.last_requested.remove(&target.key);
            }
            outcomes.push(Outcome {
                id: target.id,
                error: result.err(),
            });
        }
        Ok(outcomes)
    })
    .await
    .map_err(|e| format!("RGB 제어 작업 실패: {e}"))?
}

#[tauri::command]
async fn set_lighting(
    targets: Vec<Target>,
    settings: Lighting,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Outcome>> {
    if targets.is_empty() || targets.len() > 5 {
        return Err("제어할 장치를 1~5개 선택하세요.".into());
    }
    settings.validate()?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = state
            .lock()
            .map_err(|_| "앱 상태 오류입니다. 다시 실행하세요.")?;
        let mut seen = std::collections::HashSet::new();
        let mut outcomes = Vec::new();
        for target in targets {
            let result = if seen.insert(target.id) {
                devices::lighting::apply(target.id, &target.key, &settings)
            } else {
                Err("중복된 장치 요청입니다.".into())
            };
            if result.is_ok() {
                session
                    .lighting
                    .insert(target.key.clone(), settings.clone());
                session
                    .last_requested
                    .insert(target.key, settings.brightness > 0);
            } else {
                session.last_requested.remove(&target.key);
            }
            outcomes.push(Outcome {
                id: target.id,
                error: result.err(),
            });
        }
        Ok(outcomes)
    })
    .await
    .map_err(|e| format!("조명 설정 작업 실패: {e}"))?
}

#[tauri::command]
async fn get_mouse_dpi(key: String) -> Result<DpiProfile> {
    devices::validate_target(0, &key)?;
    tauri::async_runtime::spawn_blocking(corsair_runtime::profile)
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct DpiOutcome {
    profile: DpiProfile,
    lighting: Lighting,
    warning: Option<String>,
}

#[tauri::command]
async fn set_mouse_dpi(
    key: String,
    profile: DpiProfile,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DpiOutcome> {
    devices::validate_target(0, &key)?;
    profile.validate()?;
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("mouse-dpi.json");
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = state.lock().map_err(|_| "앱 상태 오류입니다.")?;
        let settings = session.lighting.get(&key).cloned().unwrap_or(Lighting {
            effect: devices::lighting::Effect::Static,
            color: [255, 164, 211],
            brightness: 70,
            speed: 2,
        });
        let profile = corsair_runtime::configure(profile, &settings)?;
        session.lighting.insert(key.clone(), settings.clone());
        session.last_requested.insert(key, settings.brightness > 0);
        // Commit preferences only after device application succeeds. Save failure is
        // distinct from apply failure: the current mouse settings are still active.
        let save = (|| -> Result<()> {
            std::fs::create_dir_all(path.parent().ok_or("설정 경로 오류")?)
                .map_err(|e| e.to_string())?;
            let temp = path.with_extension("json.tmp");
            std::fs::write(
                &temp,
                serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::rename(temp, path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        Ok(DpiOutcome {
            profile,
            lighting: settings,
            warning: save
                .err()
                .map(|e| format!("DPI는 적용됐지만 앱 설정 저장에 실패했습니다: {e}")),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") {
                desktop::show(app);
            }
        }))
        .manage(AppState::default())
        .manage(startup::State::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            desktop::setup(app)?;
            startup::initialize(app.handle().clone());
            let path = app.path().app_config_dir()?.join("mouse-dpi.json");
            if path.exists() {
                let load = std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        serde_json::from_slice::<DpiProfile>(&bytes).map_err(|e| e.to_string())
                    })
                    .and_then(corsair_runtime::set_saved);
                if let Err(error) = load {
                    corsair_runtime::report_error(format!(
                        "저장된 DPI 설정을 읽지 못했습니다: {error}"
                    ));
                }
            }
            let handle = app.handle().clone();
            corsair_runtime::start(move |status| {
                let _ = handle.emit("mouse-status", status);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            set_power,
            set_lighting,
            get_mouse_dpi,
            set_mouse_dpi,
            startup::get_startup,
            startup::set_startup
        ])
        .build(tauri::generate_context!())
        .expect("RGB Switch 실행 실패")
        .run(|_, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let _ = corsair_runtime::shutdown();
            }
        });
}
