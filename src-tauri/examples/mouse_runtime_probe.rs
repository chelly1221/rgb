//! Bounded runtime regression: exact wired mouse only, restores onboard state.
use rgb_switch_lib::devices::{
    corsair::Mouse,
    corsair_runtime as runtime,
    lighting::{Effect, Lighting},
    Result,
};
fn main() {
    let result = (|| -> Result<serde_json::Value> {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let original = Mouse::open(&api)?.dpi_profile()?;
        runtime::start(|status| println!("status={}", serde_json::to_string(&status).unwrap()));
        let test = (|| -> Result<()> {
            let mut settings = Lighting {
                effect: Effect::Static,
                color: [255, 80, 145],
                brightness: 70,
                speed: 2,
            };
            runtime::lighting(&settings)?;
            if runtime::inspect()? != 2 {
                return Err("Expected software mode".into());
            }
            let mut custom = original.clone();
            custom.values = [200, 1200, 1800, 5200, 10000];
            custom.colors = [
                [255, 128, 0],
                [128, 0, 255],
                [0, 255, 128],
                [255, 0, 128],
                [0, 128, 255],
            ];
            custom.index = 2;
            if runtime::configure(custom.clone(), &settings)? != custom
                || runtime::profile()? != custom
            {
                return Err("Profile update mismatch".into());
            }
            for index in 0..5 {
                custom.index = index;
                runtime::configure(custom.clone(), &settings)?;
                println!(
                    "Selected {}: current DPI readback {:?}",
                    custom.values[index as usize],
                    runtime::current_dpi()
                );
            }
            runtime::power(false)?;
            if !runtime::status()?.active {
                return Err("DPI listener stopped on LED off".into());
            }
            runtime::power(true)?;
            settings.color = [255, 190, 80];
            runtime::lighting(&settings)?;
            // Cross the heartbeat interval and exercise scans while the listener owns HID.
            for _ in 0..22 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if runtime::inspect()? != 2 {
                    return Err("Runtime lost mode".into());
                }
            }
            runtime::shutdown()?;
            if runtime::status()?.active {
                return Err("Runtime remained active after shutdown".into());
            }
            Ok(())
        })();
        let cleanup = runtime::shutdown();
        let mouse = Mouse::open(&api)?;
        let after = mouse.dpi_profile()?;
        let unchanged = original.values == after.values
            && original.colors == after.colors
            && original.mask == after.mask;
        mouse.begin_software_rgb()?;
        let selection = mouse.select_dpi(&original, original.index);
        let restore_mode = mouse.finish_software(true);
        let mode = mouse.render_mode()?;
        Ok(
            serde_json::json!({"test":test,"cleanup":cleanup,"nativePresetsUnchanged":unchanged,"selectionRestore":selection,"modeRestore":restore_mode,"finalMode":mode}),
        )
    })();
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("{json}");
    std::fs::write(
        "C:/code/control-windows-devices/tools/mouse-runtime-result.json",
        json,
    )
    .unwrap();
}
