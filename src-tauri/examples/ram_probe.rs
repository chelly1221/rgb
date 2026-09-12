fn main() {
    let result = rgb_switch_lib::devices::ram::Ram::open().and_then(|ram| ram.snapshot());
    let report = serde_json::to_string_pretty(&result).unwrap();
    std::fs::write(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/bundled-ram-result.json"),
        &report,
    )
    .unwrap();
    println!("{report}");
}
