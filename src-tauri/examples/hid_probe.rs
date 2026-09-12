use hidapi::HidApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = HidApi::new()?;
    for info in api.device_list().filter(|d| {
        matches!(
            (d.vendor_id(), d.product_id()),
            (0x1b1c, 0x1b5e) | (0x0db0, 0x0076) | (0x05ac, 0x0256)
        )
    }) {
        println!(
            "{:04x}:{:04x} interface={} usage={:04x}:{:04x} product={:?}",
            info.vendor_id(),
            info.product_id(),
            info.interface_number(),
            info.usage_page(),
            info.usage(),
            info.product_string()
        );
        if info.vendor_id() == 0x0db0 {
            match info.open_device(&api) {
                Ok(dev) => {
                    for (id, size) in [(0x52, 200), (0x50, 290), (0x50, 761)] {
                        let mut buf = vec![0u8; size];
                        buf[0] = id;
                        match dev.get_feature_report(&mut buf) {
                            Ok(n) => {
                                println!(
                                    " feature {id:02x} requested={size} received={n} head={:02x?}",
                                    &buf[..n.min(32)]
                                );
                                std::fs::write(
                                    std::env::temp_dir()
                                        .join(format!("rgb-msi-{id:02x}-{size}.bin")),
                                    &buf[..n],
                                )?;
                            }
                            Err(e) => println!(" feature {id:02x}: {e}"),
                        }
                    }
                }
                Err(e) => println!(" open: {e}"),
            }
        }
        if info.vendor_id() == 0x1b1c && info.interface_number() == 1 && info.usage_page() == 0xff42
        {
            let dev = info.open_device(&api)?;
            let mut p = [0u8; 65];
            p[1] = 8;
            p[2] = 2;
            p[3] = 0x12;
            dev.write(&p)?;
            let mut r = [0u8; 1024];
            let n = dev.read_timeout(&mut r, 500)?;
            println!(
                " PID query response length={n} bytes={:02x?}",
                &r[..n.min(16)]
            );
        }
    }
    Ok(())
}
