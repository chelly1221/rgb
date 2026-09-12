use rgb_switch_lib::devices::keyboard::Keyboard;
fn main() -> Result<(), String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let k = Keyboard::open(&api)?;
    k.power(false)?;
    println!("Keyboard OFF acknowledged; waiting 5 seconds.");
    std::thread::sleep(std::time::Duration::from_secs(5));
    k.power(true)?;
    println!("Keyboard white ON acknowledged. No flash save sent.");
    Ok(())
}
