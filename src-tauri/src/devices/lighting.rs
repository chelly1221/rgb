use super::{validate_target, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    Static,
    Breathe,
    Spectrum,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lighting {
    pub effect: Effect,
    pub color: [u8; 3],
    pub brightness: u8,
    pub speed: u8,
}
impl Lighting {
    pub fn validate(&self) -> Result<()> {
        if self.brightness > 100 || !(1..=3).contains(&self.speed) {
            return Err("밝기는 0~100%, 속도는 1~3으로 설정하세요.".into());
        }
        Ok(())
    }
    pub fn level(&self, maximum: u16) -> u8 {
        ((u16::from(self.brightness) * maximum + 50) / 100) as u8
    }
}
pub fn apply(id: u32, key: &str, settings: &Lighting) -> Result<()> {
    validate_target(id, key)?;
    settings.validate()?;
    match id {
        0 => super::corsair_runtime::lighting(settings),
        1 => super::gpu::Gpu::open()?.lighting(settings),
        2 => super::msi::Board::open(&hidapi::HidApi::new().map_err(|e| e.to_string())?)?
            .lighting(settings),
        3 => super::keyboard::Keyboard::open(&hidapi::HidApi::new().map_err(|e| e.to_string())?)?
            .lighting(settings),
        4 => super::ram::Ram::open()?.lighting(settings),
        _ => Err("이 장치는 DPI 표시를 유지하는 기본 조명 켜기·끄기를 사용하세요.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_frontend_settings_before_device_access() {
        let mut s = Lighting {
            effect: Effect::Static,
            color: [255, 90, 120],
            brightness: 101,
            speed: 2,
        };
        assert!(s.validate().is_err());
        s.brightness = 100;
        assert_eq!(s.level(255), 255);
        s.speed = 0;
        assert!(s.validate().is_err());
        assert!(serde_json::from_str::<Lighting>(
            r#"{"effect":"flashFirmware","color":[0,0,0],"brightness":100,"speed":2}"#
        )
        .is_err());
    }
}
