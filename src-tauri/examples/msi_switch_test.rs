//! Explicit OFF/ON/restore test of the exact MSI global lighting switch.
use rgb_switch_lib::devices::msi::Board;
fn main() -> Result<(), String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let board = Board::open(&api)?;
    let previous = board.enabled()?;
    let effects = board.lighting_settings()?;
    println!("MSI initial global switch: {previous}");
    let result = (|| {
        board.power(false)?;
        println!("MSI OFF acknowledged + readback OK");
        std::thread::sleep(std::time::Duration::from_secs(5));
        board.power(true)?;
        println!("MSI ON acknowledged + readback OK");
        if board.lighting_settings()? != effects {
            return Err("Lighting effects changed during test".into());
        }
        println!("All 290 lighting configuration bytes unchanged");
        Ok(())
    })();
    let restore = board.power(previous);
    println!("MSI original global switch restore: {restore:?}");
    restore?;
    result
}
