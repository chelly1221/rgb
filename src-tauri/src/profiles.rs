use crate::{
    devices::{
        self,
        corsair::DpiProfile,
        lighting::{Effect, Lighting},
        Result,
    },
    AppState, Outcome,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};
use tauri::Manager;

static STORE_LOCK: Mutex<()> = Mutex::new(());
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: u32,
    pub key: String,
    pub enabled: bool,
    pub lighting: Lighting,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub devices: Vec<Entry>,
    pub mouse_dpi: Option<DpiProfile>,
}
impl Profile {
    fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err("프로파일 식별값이 올바르지 않습니다.".into());
        }
        if self.name.trim().is_empty()
            || self.name.chars().count() > 40
            || self.name.chars().any(char::is_control)
        {
            return Err("프로파일 이름은 1~40자로 입력하세요.".into());
        }
        if self.devices.len() != 5 {
            return Err("전체 5개 장치의 설정이 필요합니다.".into());
        }
        let mut seen = std::collections::HashSet::new();
        for entry in &self.devices {
            devices::validate_target(entry.id, &entry.key)?;
            if !seen.insert(entry.id) {
                return Err("프로파일에 중복 장치가 있습니다.".into());
            }
            entry.lighting.validate()?;
            if entry.id == 0 && entry.lighting.effect != Effect::Static {
                return Err("마우스는 단색만 지원합니다.".into());
            }
        }
        if let Some(dpi) = &self.mouse_dpi {
            dpi.validate()?;
        }
        Ok(())
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    version: u32,
    profiles: Vec<Profile>,
}
fn read(path: &Path) -> Result<Vec<Profile>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    if file.metadata().map_err(|e| e.to_string())?.len() > 1_048_576 {
        return Err("프로파일 파일이 너무 큽니다.".into());
    }
    let store: Store = serde_json::from_reader(file)
        .map_err(|e| format!("프로파일 파일을 읽지 못했습니다: {e}"))?;
    if store.version != 1 || store.profiles.len() > 32 {
        return Err("지원하지 않는 프로파일 파일입니다.".into());
    }
    let mut ids = std::collections::HashSet::new();
    for profile in &store.profiles {
        profile.validate()?;
        if !ids.insert(&profile.id) {
            return Err("중복된 프로파일 식별값입니다.".into());
        }
    }
    Ok(store.profiles)
}
fn save(path: &Path, mut profile: Profile) -> Result<Vec<Profile>> {
    profile.name = profile.name.trim().to_owned();
    profile.validate()?;
    let mut profiles = read(path)?;
    if profiles
        .iter()
        .any(|p| p.id != profile.id && p.name.to_lowercase() == profile.name.to_lowercase())
    {
        return Err("같은 이름의 프로파일이 있습니다. 다른 이름을 입력하세요.".into());
    }
    if let Some(existing) = profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        if profiles.len() >= 32 {
            return Err("프로파일은 최대 32개까지 저장할 수 있습니다.".into());
        }
        profiles.push(profile);
    }
    std::fs::create_dir_all(path.parent().ok_or("프로파일 경로 오류")?)
        .map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(&Store {
        version: 1,
        profiles: profiles.clone(),
    })
    .map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(temp, path).map_err(|e| e.to_string())?;
    Ok(profiles)
}
#[tauri::command]
pub async fn list_profiles(app: tauri::AppHandle) -> Result<Vec<Profile>> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STORE_LOCK.lock().map_err(|_| "프로파일 저장소 오류")?;
        read(
            &app.path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("profiles.json"),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_profile(app: tauri::AppHandle, profile: Profile) -> Result<Vec<Profile>> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = STORE_LOCK.lock().map_err(|_| "프로파일 저장소 오류")?;
        save(
            &app.path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("profiles.json"),
            profile,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn apply_profile(
    profile: Profile,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Outcome>> {
    // Validate the entire request before any hardware writes.
    profile.validate()?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = state.lock().map_err(|_| "앱 상태 오류")?;
        let mut outcomes = Vec::new();
        for entry in profile.devices {
            let mut settings = entry.lighting.clone();
            if !entry.enabled {
                settings.brightness = 0;
            }
            let result = if entry.id == 0 {
                if let Some(dpi) = &profile.mouse_dpi {
                    devices::corsair_runtime::configure(dpi.clone(), &settings).map(|_| ())
                } else {
                    devices::lighting::apply(entry.id, &entry.key, &settings)
                }
            } else if entry.enabled {
                devices::lighting::apply(entry.id, &entry.key, &settings)
            } else {
                session.power(entry.id, &entry.key, false)
            };
            if result.is_ok() {
                session.lighting.insert(entry.key.clone(), entry.lighting);
                session
                    .last_requested
                    .insert(entry.key, entry.enabled && settings.brightness > 0);
            } else {
                session.last_requested.remove(&entry.key);
            }
            outcomes.push(Outcome {
                id: entry.id,
                error: result.err(),
            });
        }
        Ok(outcomes)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Profile {
        Profile {
            id: "test-profile".into(),
            name: "저녁".into(),
            mouse_dpi: None,
            devices: devices::KEYS
                .iter()
                .enumerate()
                .map(|(id, key)| Entry {
                    id: id as u32,
                    key: key.to_string(),
                    enabled: id != 2,
                    lighting: Lighting {
                        effect: Effect::Static,
                        color: [255, 90, 120],
                        brightness: 70,
                        speed: 2,
                    },
                })
                .collect(),
        }
    }
    #[test]
    fn invalid_profile_is_rejected_before_application() {
        let mut p = fixture();
        assert!(p.validate().is_ok());
        p.devices[1].key = "other-device".into();
        assert!(p.validate().is_err());
        p = fixture();
        p.devices[1] = p.devices[0].clone();
        assert!(p.validate().is_err());
        p = fixture();
        p.devices[0].lighting.effect = Effect::Spectrum;
        assert!(p.validate().is_err());
        p = fixture();
        p.devices[4].lighting.brightness = 101;
        assert!(p.validate().is_err());
        p = fixture();
        p.devices.pop();
        assert!(p.validate().is_err());
    }
    #[test]
    fn saving_replacing_and_corruption_preserve_prior_data() {
        let dir = std::env::temp_dir().join(format!(
            "rgb-profiles-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("profiles.json");
        assert!(read(&path).unwrap().is_empty());
        let mut p = fixture();
        save(&path, p.clone()).unwrap();
        p.name = "밤".into();
        p.devices[2].enabled = true;
        save(&path, p.clone()).unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "밤");
        assert!(loaded[0].devices[2].enabled);
        p.id = "other".into();
        assert!(save(&path, p.clone()).is_err());
        std::fs::write(&path, b"broken JSON").unwrap();
        assert!(save(&path, p).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"broken JSON");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
