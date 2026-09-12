//! Explicit 45-second static-green test of the present MSI headers, then restore.
use rgb_switch_lib::devices::{
    lighting::{Effect, Lighting},
    msi::Board,
    Result,
};
fn main() {
    let result = (|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let board = Board::open(&api)?;
        let before = board.lighting_settings()?;
        let enabled = board.enabled()?;
        let change: Result<()> = (|| {
            board.lighting(&Lighting {
                effect: Effect::Static,
                color: [0, 255, 0],
                brightness: 100,
                speed: 2,
            })?;
            let applied = board.lighting_settings()?;
            for second in 1..=45 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let now = board.lighting_settings()?;
                let diff: Vec<_> = now
                    .iter()
                    .zip(&applied)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (a, b))| (i, *b, *a))
                    .collect();
                println!(
                    "second {second} global {:?} differences {diff:?}",
                    board.enabled()
                );
            }
            Ok(())
        })();
        let restore = board.restore_lighting(&before);
        let switch = board.power(enabled);
        Ok::<_, String>(serde_json::json!({"change":change,"restore":restore,"switch":switch}))
    })();
    std::fs::write(
        "C:/code/control-windows-devices/tools/fan-visible-result.json",
        serde_json::to_string_pretty(&result).unwrap(),
    )
    .unwrap();
}
