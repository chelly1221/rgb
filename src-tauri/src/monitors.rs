//! Windows DDC/CI brightness. Physical handles never escape the blocking worker.
use serde::Serialize;
use std::{mem::size_of, ptr, sync::Mutex, thread::sleep, time::Duration};
use windows_sys::Win32::{
    Devices::Display::{
        DestroyPhysicalMonitor, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR, SetMonitorBrightness, PHYSICAL_MONITOR,
    },
    Foundation::{LPARAM, RECT},
    Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
        MONITORINFOEXW,
    },
};

static ACCESS: Mutex<()> = Mutex::new(());
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub brightness: Option<u32>,
    pub error: Option<String>,
}

struct Physical {
    handle: PHYSICAL_MONITOR,
    monitor: Monitor,
}
impl Drop for Physical {
    fn drop(&mut self) {
        unsafe {
            DestroyPhysicalMonitor(self.handle.hPhysicalMonitor);
        }
    }
}

fn wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|v| *v == 0).unwrap_or(value.len())])
}
fn os_error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}
fn edid_name(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 128 || bytes[..8] != [0, 255, 255, 255, 255, 255, 255, 0] {
        return None;
    }
    bytes[54..126].chunks_exact(18).find_map(|descriptor| {
        if descriptor[..5] != [0, 0, 0, 0xfc, 0] {
            return None;
        }
        let name = String::from_utf8_lossy(&descriptor[5..])
            .trim_matches(|c: char| c.is_whitespace() || c == '\0')
            .to_string();
        (!name.is_empty() && !name.chars().any(char::is_control)).then_some(name)
    })
}
fn friendly_name(identity: &str) -> Option<String> {
    let mut parts = identity.split('#');
    if !parts.next()?.ends_with("DISPLAY") {
        return None;
    }
    let model = parts.next()?;
    let instance = parts.next()?;
    let key = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(format!(
            "SYSTEM\\CurrentControlSet\\Enum\\DISPLAY\\{model}\\{instance}\\Device Parameters"
        ))
        .ok()?;
    edid_name(&key.get_raw_value("EDID").ok()?.bytes)
}
unsafe extern "system" fn collect(handle: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    (*(data as *mut Vec<HMONITOR>)).push(handle);
    1
}

fn enumerate() -> Result<(Vec<Physical>, Vec<Monitor>)> {
    let mut logical: Vec<HMONITOR> = Vec::new();
    if unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(collect),
            &mut logical as *mut _ as LPARAM,
        )
    } == 0
    {
        return Err(os_error("연결된 모니터 검색 실패"));
    }
    let mut physical = Vec::new();
    let mut unavailable = Vec::new();
    for handle in logical {
        let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(handle, &mut info.monitorInfo) } == 0 {
            return Err(os_error("모니터 정보 확인 실패"));
        }
        let display = wide(&info.szDevice);
        let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        device.cb = size_of::<DISPLAY_DEVICEW>() as u32;
        // Interface path prevents a re-used DISPLAY number from targeting another monitor.
        let found = unsafe { EnumDisplayDevicesW(info.szDevice.as_ptr(), 0, &mut device, 1) } != 0;
        let identity = if found {
            wide(&device.DeviceID)
        } else {
            String::new()
        };
        let base = Monitor {
            id: format!("{display}|{identity}"),
            name: if found {
                wide(&device.DeviceString)
            } else {
                display.clone()
            },
            primary: info.monitorInfo.dwFlags & 1 != 0,
            x: info.monitorInfo.rcMonitor.left,
            y: info.monitorInfo.rcMonitor.top,
            brightness: None,
            error: None,
        };
        let mut count = 0;
        if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(handle, &mut count) } == 0 || count == 0
        {
            unavailable.push(Monitor { error: Some("DDC/CI 모니터를 찾지 못했습니다. 연결과 모니터 메뉴의 DDC/CI 설정을 확인하세요.".into()), ..base });
            continue;
        }
        if count > 64 {
            return Err("모니터 개수가 올바르지 않습니다.".into());
        }
        let mut handles: Vec<PHYSICAL_MONITOR> =
            (0..count).map(|_| unsafe { std::mem::zeroed() }).collect();
        if unsafe { GetPhysicalMonitorsFromHMONITOR(handle, count, handles.as_mut_ptr()) } == 0 {
            unavailable.push(Monitor {
                error: Some(os_error("DDC/CI 연결 실패")),
                ..base
            });
            continue;
        }
        for (index, handle) in handles.into_iter().enumerate() {
            let description_buffer = handle.szPhysicalMonitorDescription;
            let description = wide(&description_buffer);
            let name = if count == 1 {
                friendly_name(&identity)
            } else {
                None
            }
            .unwrap_or_else(|| {
                if description.is_empty() {
                    base.name.clone()
                } else {
                    description
                }
            });
            physical.push(Physical {
                handle,
                monitor: Monitor {
                    id: format!("{}|{index}|{name}", base.id),
                    name,
                    ..base.clone()
                },
            });
        }
    }
    Ok((physical, unavailable))
}

fn read(device: &Physical) -> Result<(u32, u32, u32)> {
    retry_ddc(|| read_once(device))
}

// Some displays reject closely spaced DDC messages or return a transient bad
// checksum. Pace requests and retry only this idempotent brightness operation.
fn retry_ddc<T>(mut operation: impl FnMut() -> Result<T>) -> Result<T> {
    let mut result = Err("모니터 응답이 없습니다.".into());
    for delay in [120, 250, 500] {
        sleep(Duration::from_millis(delay));
        result = operation();
        if result.is_ok() {
            break;
        }
    }
    result
}

fn read_once(device: &Physical) -> Result<(u32, u32, u32)> {
    let (mut min, mut current, mut max) = (0, 0, 0);
    if unsafe {
        GetMonitorBrightness(
            device.handle.hPhysicalMonitor,
            &mut min,
            &mut current,
            &mut max,
        )
    } == 0
    {
        return Err(os_error(
            "밝기를 읽지 못했습니다. 모니터의 DDC/CI 설정과 HDR·자동 밝기 모드를 확인하세요",
        ));
    }
    percent(min, current, max)?;
    Ok((min, current, max))
}
fn percent(min: u32, current: u32, max: u32) -> Result<u32> {
    if max <= min || current < min || current > max {
        return Err("모니터가 올바르지 않은 밝기 범위를 응답했습니다.".into());
    }
    Ok(((u64::from(current - min) * 100 + u64::from(max - min) / 2) / u64::from(max - min)) as u32)
}
fn raw(min: u32, max: u32, brightness: u32) -> Result<u32> {
    if brightness > 100 || max <= min {
        return Err("밝기 범위가 올바르지 않습니다.".into());
    }
    Ok(min + ((u64::from(max - min) * u64::from(brightness) + 50) / 100) as u32)
}
fn snapshot(device: &Physical) -> Monitor {
    let mut monitor = device.monitor.clone();
    match read(device).and_then(|(min, current, max)| percent(min, current, max)) {
        Ok(value) => monitor.brightness = Some(value),
        Err(error) => monitor.error = Some(error),
    }
    monitor
}

pub fn scan() -> Result<Vec<Monitor>> {
    let _access = ACCESS.lock().map_err(|_| "모니터 제어 상태 오류")?;
    let (devices, mut result) = enumerate()?;
    result.extend(devices.iter().map(snapshot));
    result.sort_by_key(|monitor| (monitor.x, monitor.y, monitor.id.clone()));
    Ok(result)
}

pub fn set(ids: Vec<String>, brightness: u32) -> Result<Vec<Monitor>> {
    if brightness > 100 || ids.is_empty() || ids.len() > 64 {
        return Err("모니터와 0~100 사이의 밝기를 지정하세요.".into());
    }
    let _access = ACCESS.lock().map_err(|_| "모니터 제어 상태 오류")?;
    let (devices, _) = enumerate()?;
    let mut result = Vec::new();
    for id in ids {
        if result.iter().any(|monitor: &Monitor| monitor.id == id) {
            continue;
        }
        let Some(device) = devices.iter().find(|device| device.monitor.id == id) else {
            result.push(Monitor {
                id,
                name: "연결 해제된 모니터".into(),
                primary: false,
                x: 0,
                y: 0,
                brightness: None,
                error: Some("연결이 변경되었습니다. 모니터를 다시 검색하세요.".into()),
            });
            continue;
        };
        let apply = (|| {
            let (min, _, max) = read(device)?;
            let target = raw(min, max, brightness)?;
            retry_ddc(|| {
                if unsafe { SetMonitorBrightness(device.handle.hPhysicalMonitor, target) } == 0 {
                    Err(os_error("밝기 적용 실패"))
                } else {
                    Ok(())
                }
            })?;
            percent(min, target, max)
        })();
        let mut monitor = snapshot(device);
        match apply {
            Err(error) => monitor.error = Some(error),
            Ok(expected)
                if monitor
                    .brightness
                    .is_some_and(|value| value.abs_diff(expected) > 1) =>
            {
                monitor.error = Some(
                    "요청한 밝기와 모니터 응답이 다릅니다. HDR·자동 밝기 설정을 확인하세요.".into(),
                );
            }
            Ok(_) => {}
        }
        result.push(monitor);
    }
    Ok(result)
}

#[tauri::command]
pub async fn scan_monitors() -> Result<Vec<Monitor>> {
    tauri::async_runtime::spawn_blocking(scan)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn set_monitor_brightness(ids: Vec<String>, brightness: u32) -> Result<Vec<Monitor>> {
    tauri::async_runtime::spawn_blocking(move || set(ids, brightness))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scales_nonzero_and_large_ranges_without_overflow() {
        assert_eq!(raw(20, 220, 50).unwrap(), 120);
        assert_eq!(percent(20, 120, 220).unwrap(), 50);
        assert_eq!(raw(0, u32::MAX, 100).unwrap(), u32::MAX);
        assert_eq!(percent(0, u32::MAX, u32::MAX).unwrap(), 100);
    }
    #[test]
    fn rejects_invalid_ranges_before_writing() {
        assert!(raw(0, 100, 101).is_err());
        assert!(raw(10, 10, 50).is_err());
        assert!(percent(20, 10, 100).is_err());
        assert!(percent(0, 101, 100).is_err());
        assert!(set(vec![], 50).is_err());
    }
}
