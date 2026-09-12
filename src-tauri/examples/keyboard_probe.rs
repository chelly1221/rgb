use hidapi::HidApi;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    for info in api
        .device_list()
        .filter(|d| d.vendor_id() == 0x05ac && d.product_id() == 0x0256)
    {
        match info.open_device(&api) {
            Ok(d) => {
                let mut b = [0; 4096];
                match d.get_report_descriptor(&mut b) {
                    Ok(n) => println!(
                        "iface {} usage {:04x}:{:04x}: {:02x?}",
                        info.interface_number(),
                        info.usage_page(),
                        info.usage(),
                        &b[..n]
                    ),
                    Err(e) => println!("descriptor: {e}"),
                }
            }
            Err(e) => println!("open: {e}"),
        }
    }
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == 0x05ac
                && d.product_id() == 0x0256
                && d.interface_number() == 0
                && d.usage_page() == 1
                && d.usage() == 6
                && d.product_string() == Some("GS3087T")
        })
        .ok_or("GS3087T not found")?;
    std::fs::write(
        std::env::temp_dir().join("rgb-switch-research/keyboard-path.txt"),
        info.path().to_bytes(),
    )?;
    let d = info.open_device(&api)?;
    let mut descriptor = [0; 4096];
    println!("Descriptor: {:?}", d.get_report_descriptor(&mut descriptor));
    let mut p = [0u8; 65];
    println!("Current feature: {:?}", d.get_feature_report(&mut p));
    p = [0; 65];
    p[1] = 4;
    p[2] = 0xf5;
    p[9] = 9;
    d.send_feature_report(&p)?;
    std::thread::sleep(std::time::Duration::from_millis(20));
    for i in 0..9 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut r = [0; 65];
        match d.get_feature_report(&mut r) {
            Ok(n) => println!("Custom light query {i} n={n}: {:02x?}", &r[..n]),
            Err(e) => {
                println!("Read failed {e}");
                break;
            }
        }
    }
    p = [0; 65];
    p[1] = 4;
    p[2] = 2;
    d.send_feature_report(&p)?;
    std::thread::sleep(std::time::Duration::from_millis(20));
    let mut close = [0; 65];
    println!("Close {:?}", d.get_feature_report(&mut close));
    Ok(())
}
