//! Explicit hardware regression of the app's MSI power and native effect paths.
//! Finishes with the fan lighting OFF, retaining the pre-run snapshot on disk.
use rgb_switch_lib::devices::{
    lighting::{Effect, Lighting},
    msi::{Board, PowerState},
};
fn main() {
    let result = (|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let board = Board::open(&api)?;
        let before = board.lighting_settings()?;
        let enabled = board.enabled()?;
        std::fs::write(
            "C:/code/control-windows-devices/tools/msi-regression-before-result.json",
            serde_json::to_vec(&(before.clone(), enabled)).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        let check = (|| {
            let mut state = PowerState::default();
            board.set_power(true, &mut state)?;
            let on = board.lighting_settings()?;
            std::thread::sleep(std::time::Duration::from_secs(3));
            board.set_power(false, &mut state)?;
            require_off(&board)?;
            board.set_power(false, &mut state)?;
            board.set_power(true, &mut state)?;
            if board.lighting_settings()?[..289] != on[..289] {
                return Err("Repeated OFF lost the original ON effect".into());
            }
            for effect in [Effect::Static, Effect::Breathe, Effect::Spectrum] {
                board.lighting(&Lighting {
                    effect,
                    color: [255, 70, 120],
                    brightness: 60,
                    speed: 2,
                })?;
                let applied = board.lighting_settings()?;
                board.set_power(false, &mut state)?;
                require_off(&board)?;
                board.set_power(true, &mut state)?;
                if board.lighting_settings()?[..289] != applied[..289] {
                    return Err(format!("Effect {effect:?} did not resume exactly"));
                }
            }
            board.lighting(&Lighting {
                effect: Effect::Static,
                color: [255; 3],
                brightness: 0,
                speed: 2,
            })?;
            require_off(&board)?;
            Ok::<_, String>(
                serde_json::json!({"power":true,"repeatedOff":true,"staticResume":true,"breathingResume":true,"spectrumResume":true,"zeroBrightnessOff":true}),
            )
        })();
        if check.is_err() {
            let restore = board.restore_lighting(&before);
            let switch = board.power(enabled);
            return Err(format!("{check:?}; restore {restore:?}; switch {switch:?}"));
        }
        check
    })();
    std::fs::write(
        "C:/code/control-windows-devices/tools/msi-regression-result.json",
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
fn require_off(board: &Board) -> Result<(), String> {
    let data = board.lighting_settings()?;
    if !board.enabled()?
        || [0, 1, 2, 3, 9, 17]
            .iter()
            .any(|i| data[1 + i * 16..14 + i * 16].iter().any(|v| *v != 0))
    {
        return Err("Native Off readback failed".into());
    }
    Ok(())
}
