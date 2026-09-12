//! Read-only MB800 protocol discovery; only documented GET requests.
use hidapi::HidApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    let candidates: Vec<_> = api
        .device_list()
        .filter(|d| {
            d.vendor_id() == 0x0db0
                && d.product_id() == 0x0076
                && d.interface_number() == 0
                && d.usage_page() == 0xff00
                && d.usage() == 1
        })
        .collect();
    if candidates.len() != 1 {
        return Err("Expected one exact MSI interface".into());
    }
    let info = candidates[0];
    println!(
        "Board serial model prefix matches 7E40: {}",
        info.serial_number().is_some_and(|s| s.starts_with("7E40"))
    );
    let dev = info.open_device(&api)?;
    for (id, size) in [
        (0x50, 290),
        (0x90, 302),
        (0x91, 302),
        (0x92, 302),
        (0x93, 302),
    ] {
        let mut r = vec![0xff; size];
        r[0] = id;
        match dev.get_feature_report(&mut r) {
            Ok(n) => {
                println!("Feature {id:02x} len={n}, head={:02x?}", &r[..n.min(32)]);
                std::fs::write(
                    std::env::temp_dir().join(format!("rgb-msi-{id:02x}-baseline.bin")),
                    &r[..n],
                )?;
            }
            Err(e) => println!("Feature {id:02x}: {e}"),
        }
    }
    for command in [0xb0, 0xba] {
        for _ in 0..16 {
            let mut stale = [0; 64];
            if dev.read_timeout(&mut stale, 2)? == 0 {
                break;
            }
        }
        let mut request = [0u8; 64];
        if command == 0xb0 {
            request.fill(0xcc);
        }
        request[0] = 1;
        request[1] = command;
        println!("GET {command:02x} wrote {}", dev.write(&request)?);
        let mut reply = [0; 64];
        let n = dev.read_timeout(&mut reply, 1000)?;
        println!("GET {command:02x} reply n={n} {:02x?}", &reply[..n.min(16)]);
    }
    Ok(())
}
