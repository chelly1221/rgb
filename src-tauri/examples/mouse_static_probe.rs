//! Bounded logo/DPI test. Only current DPI selection changes; presets stay intact.
fn main() {
    let result = (|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let mouse = rgb_switch_lib::devices::corsair::Mouse::open(&api)?;
        let initial = mouse.dpi_profile()?;
        let mut listener = rgb_switch_lib::devices::corsair::DpiListener::open(&api)?;
        let test = (|| {
            mouse.begin_software_rgb()?;
            let mut index = initial.index;
            mouse.software_frame([255, 80, 145], initial.colors[index as usize])?;
            let mut beat = std::time::Instant::now();
            let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
            println!(
                "READY: pink logo, current DPI {}",
                initial.values[index as usize]
            );
            while std::time::Instant::now() < until {
                if listener.pressed()? {
                    index = rgb_switch_lib::devices::corsair::next_dpi(initial.mask, index)?;
                    mouse.select_dpi(&initial, index)?;
                    mouse.software_frame([255, 80, 145], initial.colors[index as usize])?;
                    println!(
                        "DPI button: stage={}, value={}, color={:?}",
                        index + 1,
                        initial.values[index as usize],
                        initial.colors[index as usize]
                    );
                }
                if beat.elapsed().as_secs() >= 10 {
                    mouse.heartbeat()?;
                    beat = std::time::Instant::now();
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok::<_, String>(())
        })();
        let restore = mouse.finish_software(true);
        let after = mouse.dpi_profile()?;
        Ok::<_, String>(
            serde_json::json!({"test":test,"restore":restore,"presetsUnchanged":initial.values==after.values && initial.colors==after.colors && initial.mask==after.mask}),
        )
    })();
    std::fs::write(
        "C:/code/control-windows-devices/tools/mouse-static-result.json",
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
