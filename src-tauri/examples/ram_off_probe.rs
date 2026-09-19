use rgb_switch_lib::devices::{
    lighting::{Effect, Lighting},
    ram::{PowerState, Ram},
};
fn main() {
    let result = (|| -> Result<serde_json::Value, String> {
        let ram = Ram::open()?;
        let before = ram.snapshot()?;
        let verified_off = vec![1, 1, 1, 0, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut state = PowerState::default();
        for _ in 0..2 {
            ram.power_with_state(false, &mut state)?;
            if ram.snapshot()?.iter().any(|p| *p != verified_off) {
                return Err("Power OFF differs from the physically verified packet".into());
            }
        }
        for effect in [Effect::Static, Effect::Breathe, Effect::Spectrum] {
            ram.lighting(&Lighting {
                effect,
                color: [255, 164, 211],
                brightness: 0,
                speed: 2,
            })?;
            if ram.snapshot()?.iter().any(|p| *p != verified_off) {
                return Err("Zero brightness differs from the physically verified packet".into());
            }
        }
        let after = ram.snapshot()?;
        // Leave the requested OFF in place; release the bus during the observation interval.
        drop(ram);
        std::thread::sleep(std::time::Duration::from_secs(8));
        let later = Ram::open()?.snapshot()?;
        Ok(serde_json::json!({"before":before,"after":after,"later":later}))
    })();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/ram-off-result.json");
    std::fs::write(path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
}
