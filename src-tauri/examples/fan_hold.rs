use rgb_switch_lib::devices::{
    lighting::{Effect, Lighting},
    msi::Board,
};
fn main() {
    let command = std::env::args().nth(1).unwrap_or_default();
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let result = (|| {
        if !matches!(
            command.as_str(),
            "restore" | "black" | "dim-local" | "vendor-off" | "gen1-off" | "status"
        ) {
            return Err(
                "Specify black, dim-local, vendor-off, gen1-off, restore, or status".into(),
            );
        }
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let board = Board::open(&api)?;
        let snapshot = "C:/code/control-windows-devices/tools/fan-snapshot-result.json";
        if command == "restore" {
            let (data, on): (Vec<u8>, bool) =
                serde_json::from_slice(&std::fs::read(snapshot).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            board.restore_lighting(&data)?;
            board.power(on)?;
        } else if matches!(
            command.as_str(),
            "black" | "dim-local" | "vendor-off" | "gen1-off"
        ) {
            if !std::path::Path::new(snapshot).exists() {
                std::fs::write(
                    snapshot,
                    serde_json::to_vec(&(board.lighting_settings()?, board.enabled()?)).unwrap(),
                )
                .map_err(|e| e.to_string())?;
            }
            if command == "gen1-off" {
                board.prepare_legacy_headers()?;
            }
            if command == "vendor-off" || command == "gen1-off" {
                // Installed MB800 SetOFF: Off, four black slots, one color,
                // SelectAll, custom color, B100, medium speed, cycle count 30.
                // Only the six areas present on this board are changed.
                let previous = board.lighting_settings()?;
                let was_on = board.enabled()?;
                let mut data = previous.clone();
                for i in [0, 1, 2, 3, 9, 17] {
                    let a = 1 + i * 16;
                    data[a..a + 16].fill(0);
                    data[a + 14] = 0xb5;
                    // Readback stores this field only on the four ARGB ports.
                    // Board::restore_lighting supplies the separately captured
                    // outbound value 255 for JRGB1 and SelectAll.
                    data[a + 15] = if i < 4 { 30 } else { 0 };
                }
                // Keep the controller driving the headers during native Off.
                let apply = board
                    .power(true)
                    .and_then(|_| board.restore_lighting(&data));
                if let Err(error) = apply {
                    let restore = board.restore_lighting(&previous);
                    let switch = board.power(was_on);
                    return Err(format!("{error}; restore: {restore:?}, switch: {switch:?}"));
                }
            } else {
                board.lighting(&Lighting {
                    effect: Effect::Static,
                    color: [0, 0, 0],
                    brightness: 100,
                    speed: 2,
                })?;
            }
            if command == "dim-local" {
                let mut data = board.lighting_settings()?;
                for i in [0, 1, 2, 3, 9, 17] {
                    let a = 1 + i * 16;
                    data[a] = 2;
                    data[a + 1..a + 13].fill(255);
                    data[a + 14] = (data[a + 14] & 0x40) | 0x20 | 1; // independent, custom white, native brightness B0
                }
                board.restore_lighting(&data)?;
            }
        }
        Ok::<_, String>(serde_json::json!({
            "global": board.enabled()?,
            "settings": board.lighting_settings()?,
        }))
    })();
    std::fs::write(
        "C:/code/control-windows-devices/tools/fan-hold-result.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"command":command,"startedUnixMs":started,"result":result}),
        )
        .unwrap(),
    )
    .unwrap();
}
