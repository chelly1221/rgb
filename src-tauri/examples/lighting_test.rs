//! One explicit, bounded native-effect smoke test; snapshot/restore where readable.
use rgb_switch_lib::devices::{
    self,
    lighting::{Effect, Lighting},
    Result,
};
fn main() {
    let mut outcomes = Vec::<(String, Result<serde_json::Value>)>::new();
    let selected: Vec<u8> = std::env::args()
        .skip(1)
        .map(|v| v.parse::<u8>().expect("device id 1..4"))
        .collect();
    let selected = if selected.is_empty() {
        vec![1, 2, 4, 3]
    } else {
        selected
    };
    for id in selected {
        assert!((1..=4).contains(&id), "device id 1..4");
        let result = test(id);
        outcomes.push((format!("device-{id}"), result));
    }
    outcomes.push(("mouse-native-on".into(), (|| {
        let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
        let m = devices::corsair::Mouse::open(&api)?;
        let before = m.inspect_lighting();
        let off = m.power(false);
        std::thread::sleep(std::time::Duration::from_millis(700));
        m.power(true)?;
        Ok(serde_json::json!({"before":before,"after":m.inspect_lighting(),"off":off,"renderMode":m.render_mode()?}))
    })()));
    std::fs::write(
        "C:/code/control-windows-devices/tools/lighting-test-result.json",
        serde_json::to_string_pretty(&outcomes).unwrap(),
    )
    .unwrap();
}
fn test(id: u8) -> Result<serde_json::Value> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let gpu = if id == 1 {
        Some(devices::gpu::Gpu::open()?)
    } else {
        None
    };
    let board = if id == 2 {
        Some(devices::msi::Board::open(&api)?)
    } else {
        None
    };
    let ram = if id == 4 {
        Some(devices::ram::Ram::open()?)
    } else {
        None
    };
    let keyboard = if id == 3 {
        Some(devices::keyboard::Keyboard::open(&api)?)
    } else {
        None
    };
    let gpu_before = gpu.as_ref().map(|g| g.effect_snapshot()).transpose()?;
    let board_before = board
        .as_ref()
        .map(|b| Ok::<_, String>((b.lighting_settings()?, b.enabled()?)))
        .transpose()?;
    let ram_before = ram.as_ref().map(|r| r.snapshot()).transpose()?;
    let mut logs = Vec::new();
    for effect in [Effect::Static, Effect::Breathe, Effect::Spectrum] {
        let s = Lighting {
            effect,
            color: [255, 90, 140],
            brightness: 60,
            speed: 2,
        };
        let result = if let Some(g) = &gpu {
            g.lighting(&s)
        } else if let Some(b) = &board {
            b.lighting(&s)
        } else if let Some(r) = &ram {
            r.lighting(&s)
        } else {
            keyboard.as_ref().unwrap().lighting(&s)
        };
        let stop = result.is_err();
        std::thread::sleep(std::time::Duration::from_millis(700));
        let colors = keyboard.as_ref().map(|k| k.colors());
        logs.push(serde_json::json!({"effect":effect,"result":result,"keyboardColors":colors}));
        if stop {
            break;
        }
    }
    let restore = if let Some(g) = &gpu {
        g.restore_effect(gpu_before.as_ref().unwrap())
    } else if let Some(b) = &board {
        let (settings, enabled) = board_before.unwrap();
        let r = b.restore_lighting(&settings);
        let p = b.power(enabled);
        r.and(p)
    } else if let Some(r) = &ram {
        r.restore(ram_before.as_ref().unwrap())
    } else {
        keyboard.unwrap().power(true)
    };
    Ok(serde_json::json!({"effects":logs,"restore":restore}))
}
