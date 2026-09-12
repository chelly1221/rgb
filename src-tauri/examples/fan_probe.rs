fn main() {
    let result = (|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let board = rgb_switch_lib::devices::msi::Board::open(&api)?;
        Ok::<_, String>(
            serde_json::json!({"portMode":board.inspect_port_mode(),"global":board.enabled()?,"gen1":board.lighting_settings()?,"detected":board.detect_argb_ports(),"ports":board.inspect_argb_ports()}),
        )
    })();
    std::fs::write(
        "C:/code/control-windows-devices/tools/fan-probe-result.json",
        serde_json::to_string_pretty(&result).unwrap(),
    )
    .unwrap();
}
