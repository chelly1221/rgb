pub mod corsair;
pub mod corsair_runtime;
mod driver_bundle;
pub mod gpu;
pub mod keyboard;
pub mod lighting;
mod lock;
pub mod msi;
pub mod ram;

use hidapi::HidApi;
use serde::Serialize;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: u32,
    pub key: String,
    pub name: String,
    pub vendor: String,
    pub kind: u32,
    pub led_count: usize,
    pub mode: String,
    pub supported: bool,
    pub present: bool,
    pub detail: String,
    pub last_requested: Option<bool>,
    pub effects: Vec<lighting::Effect>,
    pub last_lighting: Option<lighting::Lighting>,
}

const KEYS: [&str; 5] = [
    "corsair-harpoon-wired-1b1c-1b5e",
    "palit-rtx3070-10de-2484-1569-2484",
    "msi-b860m-mortar-7e40",
    "ganss-gs3087t-05ac-0256",
    "corsair-cmh128gx5m2b6400c42-pair",
];

pub fn scan() -> Result<Vec<Device>> {
    let api = HidApi::new().map_err(|e| format!("Windows HID 장치 목록을 읽지 못했습니다: {e}"))?;
    let mouse_present = api.device_list().any(corsair::matches);
    let mouse = corsair_runtime::inspect();
    let gpu = gpu::Gpu::open().and_then(|g| g.snapshot());
    let board_present = msi::board_matches();
    let board = msi::Board::open(&api).and_then(|b| b.enabled());
    let keyboard_present = keyboard::present(&api);
    let keyboard = keyboard::Keyboard::open(&api);
    let memory = ram::Ram::open().and_then(|r| r.snapshot());
    Ok(vec![
        Device {
            id: 0,
            key: KEYS[0].into(),
            name: "CORSAIR HARPOON RGB WIRELESS".into(),
            vendor: "CORSAIR".into(),
            kind: 6,
            led_count: 2,
            mode: "USB HID · 유선".into(),
            supported: mouse.is_ok(),
            present: mouse_present,
            detail: mouse.map_or_else(
                |e| e,
                |m| format!("직접 통신 확인 · 렌더 모드 {m}. 앱 실행 중 로고 단색과 DPI 단계별 감도·표시등을 별도로 제어합니다. iCUE를 종료하고 사용하세요."),
            ),
            last_requested: None,
            effects: vec![lighting::Effect::Static],
            last_lighting: None,
        },
        Device {
            id: 1,
            key: KEYS[1].into(),
            name: "Palit GeForce RTX 3070".into(),
            vendor: "Palit / NVIDIA".into(),
            kind: 1,
            led_count: 1,
            mode: "NVIDIA 드라이버 · I²C".into(),
            supported: gpu.is_ok(),
            present: gpu.is_ok(),
            detail: gpu.map_or_else(
                |e| e,
                |_| "조명 레지스터 읽기 확인. 단색·숨쉬기·색상 순환을 설정할 수 있습니다.".into(),
            ),
            last_requested: None,
            effects: vec![lighting::Effect::Static, lighting::Effect::Breathe, lighting::Effect::Spectrum],
            last_lighting: None,
        },
        Device {
            id: 2,
            key: KEYS[2].into(),
            name: "MSI B860M MORTAR · RGB 헤더".into(),
            vendor: "MSI".into(),
            kind: 0,
            led_count: 0,
            mode: "USB HID · 조명 스위치".into(),
            supported: board.is_ok(),
            present: board_present,
            detail: board.map_or_else(
                |e| e,
                |_| "직접 통신 확인. Gen1 ARGB 모드를 초기화한 뒤 RGB 헤더의 색상·효과·꺼짐을 함께 적용합니다.".into(),
            ),
            last_requested: None,
            effects: vec![lighting::Effect::Static, lighting::Effect::Breathe, lighting::Effect::Spectrum],
            last_lighting: None,
        },
        Device {
            id: 3,
            key: KEYS[3].into(),
            name: "GS3087T".into(),
            vendor: "GANSS".into(),
            kind: 5,
            led_count: 88,
            mode: "USB HID · 유선".into(),
            supported: keyboard.is_ok(),
            present: keyboard_present,
            detail: keyboard.map_or_else(|e| e, |_| "유선 설정 인터페이스 확인. 단색·숨쉬기·색상 순환을 지원합니다. 조명 설정은 영구 저장하지 않습니다.".into()),
            last_requested: None,
            effects: vec![lighting::Effect::Static, lighting::Effect::Breathe, lighting::Effect::Spectrum],
            last_lighting: None,
        },
        Device {
            id: 4,
            key: KEYS[4].into(),
            name: "VENGEANCE RGB DDR5 · 2개".into(),
            vendor: "CORSAIR".into(),
            kind: 2,
            led_count: 20,
            mode: "SMBus · 128 GB".into(),
            supported: memory.is_ok(),
            present: memory.is_ok(),
            detail: memory.map_or_else(|e| e, |_| "64 GB 메모리 두 개의 색상·효과를 함께 설정합니다. 앱에 내장된 드라이버를 사용하며 관리자 권한이 필요합니다.".into()),
            last_requested: None,
            effects: vec![lighting::Effect::Static, lighting::Effect::Breathe, lighting::Effect::Spectrum],
            last_lighting: None,
        },
    ])
}

pub fn power(id: u32, key: &str, enabled: bool, motherboard: &mut msi::PowerState) -> Result<()> {
    validate_target(id, key)?;
    match id {
        0 => corsair_runtime::power(enabled),
        1 => {
            let gpu = gpu::Gpu::open()?;
            let previous = gpu.snapshot()?;
            match gpu.power(enabled, None) {
                Ok(()) => Ok(()),
                Err(e) => {
                    let rollback = gpu.apply(&previous);
                    Err(match rollback {
                        Ok(()) => format!("{e} 이전 설정으로 복원했습니다."),
                        Err(r) => format!("{e} 복원도 실패했습니다: {r}"),
                    })
                }
            }
        }
        2 => msi::Board::open(&HidApi::new().map_err(|e| e.to_string())?)?
            .set_power(enabled, motherboard),
        3 => keyboard::Keyboard::open(&HidApi::new().map_err(|e| e.to_string())?)?.power(enabled),
        4 => ram::Ram::open()?.power(enabled),
        _ => unreachable!(),
    }
}

pub(crate) fn validate_target(id: u32, key: &str) -> Result<()> {
    if KEYS
        .get(id as usize)
        .is_some_and(|expected| *expected == key)
    {
        Ok(())
    } else {
        Err("알 수 없는 장치 요청입니다. 목록을 새로고침하세요.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cannot_redirect_device_or_supply_arbitrary_path() {
        assert!(validate_target(0, KEYS[0]).is_ok());
        assert!(validate_target(0, KEYS[1]).is_err());
        assert!(validate_target(400, "C:/some/device").is_err());
    }
}
