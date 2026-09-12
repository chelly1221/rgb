fn main() -> Result<(), String> {
    let devices = rgb_switch_lib::devices::scan()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&devices).map_err(|e| e.to_string())?
    );
    if std::env::args().any(|a| a == "--test-gpu") {
        let gpu = rgb_switch_lib::devices::gpu::Gpu::open()?;
        let saved = gpu.snapshot()?;
        let result = (|| {
            gpu.power(false, None)?;
            println!("GPU OFF readback OK");
            std::thread::sleep(std::time::Duration::from_millis(500));
            gpu.power(true, None)?;
            println!("GPU ON readback OK");
            Ok::<_, String>(())
        })();
        let restore = gpu.apply(&saved);
        println!("GPU restore: {restore:?}");
        restore?;
        result?;
    }
    if std::env::args().any(|a| a == "--test-mouse") {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let mouse = rgb_switch_lib::devices::corsair::Mouse::open(&api)?;
        mouse.power(false)?;
        println!("Mouse OFF acknowledged");
        std::thread::sleep(std::time::Duration::from_millis(500));
        mouse.power(true)?;
        println!("Mouse ON acknowledged (white)");
    }
    Ok(())
}
