fn main() {
    let api = hidapi::HidApi::new().unwrap();
    for d in api
        .device_list()
        .filter(|d| d.vendor_id() == 0x1b1c && d.product_id() == 0x1b5e)
    {
        println!(
            "interface {} usage {:x}:{:x}",
            d.interface_number(),
            d.usage_page(),
            d.usage()
        );
    }
    let result = hidapi::HidApi::new()
        .map_err(|e| e.to_string())
        .and_then(|api| rgb_switch_lib::devices::corsair::Mouse::open(&api))
        .map(|m| m.inspect_lighting());
    std::fs::write(
        "C:/code/control-windows-devices/tools/mouse-properties-result.json",
        serde_json::to_string_pretty(&result).unwrap(),
    )
    .unwrap();
}
