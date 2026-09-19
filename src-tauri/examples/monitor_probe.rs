fn main() {
    match rgb_switch_lib::monitors::scan() {
        Ok(monitors) => {
            println!("{}", serde_json::to_string_pretty(&monitors).unwrap());
            let args: Vec<String> = std::env::args().collect();
            if let Some(index) = args.iter().position(|arg| arg == "--brightness") {
                let value: u32 = args
                    .get(index + 1)
                    .expect("brightness required")
                    .parse()
                    .expect("integer required");
                let result = rgb_switch_lib::monitors::set(
                    monitors.iter().map(|monitor| monitor.id.clone()).collect(),
                    value,
                )
                .expect("brightness failed");
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
                return;
            }
            if std::env::args().any(|arg| arg == "--verify") {
                for monitor in monitors {
                    let Some(original) = monitor.brightness else {
                        continue;
                    };
                    let target = if original == 100 { 99 } else { original + 1 };
                    let changed = rgb_switch_lib::monitors::set(vec![monitor.id.clone()], target);
                    // Always restore, including when a write or readback fails.
                    let restored = rgb_switch_lib::monitors::set(vec![monitor.id], original);
                    for (label, result) in [("change", changed), ("restore", restored)] {
                        match result {
                            Ok(values) => {
                                println!("{label}: {}", serde_json::to_string(&values).unwrap())
                            }
                            Err(error) => eprintln!("{label}: {error}"),
                        }
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
