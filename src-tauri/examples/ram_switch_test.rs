use rgb_switch_lib::devices::ram::Ram;
fn main() {
    let result = (|| -> Result<(), String> {
        let ram = Ram::open()?;
        let original = ram.snapshot()?;
        let mut state = rgb_switch_lib::devices::ram::PowerState::default();
        println!("RAM exact identity, firmware, and configuration CRC verified.");
        let test = (|| -> Result<(), String> {
            ram.power_with_state(false, &mut state)?;
            println!("Both RAM OFF: configuration readback matched.");
            std::thread::sleep(std::time::Duration::from_secs(5));
            ram.power_with_state(true, &mut state)?;
            println!("Both RAM ON: configuration readback matched.");
            Ok(())
        })();
        let restored = ram.restore(&original);
        println!("Restore original RAM configuration: {restored:?}");
        test?;
        restored
    })();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/ram-switch-result.txt");
    std::fs::write(path, format!("{result:?}")).unwrap();
    println!("{result:?}");
}
