use std::mem::size_of;

use windows::{
    core::PCWSTR,
    Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP,
        DISPLAY_DEVICE_PRIMARY_DEVICE,
    },
};

#[derive(Debug)]
pub struct DisplayAdapter {
    pub index: u32,
    pub info: DisplayDeviceInfo,
    pub monitors: Vec<DisplayMonitor>,
}

#[derive(Debug)]
pub struct DisplayMonitor {
    pub index: u32,
    pub info: DisplayDeviceInfo,
}

#[derive(Debug)]
pub struct DisplayDeviceInfo {
    pub device_name: String,
    pub device_string: String,
    pub device_id: String,
    pub device_key: String,
    pub is_primary: bool,
    pub is_attached_to_desktop: bool,
}

pub fn enumerate_display_adapters() -> Vec<DisplayAdapter> {
    let mut adapters = Vec::new();
    let mut adapter_index = 0_u32;

    loop {
        let Some(raw_adapter) = enum_display_device(None, adapter_index) else {
            break;
        };

        // Keep the exact UTF-16 adapter name for the child-monitor calls. Converting
        // it to a Rust String first could alter malformed UTF-16 code units.
        let adapter_device_name = nul_terminated_copy(&raw_adapter.DeviceName);
        let monitors = enumerate_monitors(&adapter_device_name);

        adapters.push(DisplayAdapter {
            index: adapter_index,
            info: DisplayDeviceInfo::from_raw(&raw_adapter),
            monitors,
        });

        let Some(next_index) = adapter_index.checked_add(1) else {
            break;
        };
        adapter_index = next_index;
    }

    adapters
}

fn enumerate_monitors(adapter_device_name: &[u16]) -> Vec<DisplayMonitor> {
    let mut monitors = Vec::new();
    let mut monitor_index = 0_u32;

    loop {
        let Some(raw_monitor) =
            enum_display_device(Some(adapter_device_name), monitor_index)
        else {
            break;
        };

        monitors.push(DisplayMonitor {
            index: monitor_index,
            info: DisplayDeviceInfo::from_raw(&raw_monitor),
        });

        let Some(next_index) = monitor_index.checked_add(1) else {
            break;
        };
        monitor_index = next_index;
    }

    monitors
}

fn enum_display_device(
    parent_device_name: Option<&[u16]>,
    index: u32,
) -> Option<DISPLAY_DEVICEW> {
    let parent_device_name = match parent_device_name {
        None => PCWSTR::null(),
        Some(device_name) => {
            assert_eq!(
                device_name.last(),
                Some(&0),
                "device name must be NUL-terminated"
            );
            PCWSTR::from_raw(device_name.as_ptr())
        }
    };

    let mut device = DISPLAY_DEVICEW::default();
    device.cb = u32::try_from(size_of::<DISPLAY_DEVICEW>())
        .expect("DISPLAY_DEVICEW size must fit in a u32");

    // SAFETY: `device` is a valid, aligned, writable DISPLAY_DEVICEW and its `cb`
    // field is initialized to the exact structure size for every call. The input is
    // either NULL (adapter enumeration) or points to a NUL-terminated UTF-16 slice
    // that remains alive for the full call. EnumDisplayDevicesW does not retain
    // either pointer after it returns. `dwFlags` is zero, so this is read-only.
    let succeeded = unsafe { EnumDisplayDevicesW(parent_device_name, index, &mut device, 0) };

    succeeded.as_bool().then_some(device)
}

impl DisplayDeviceInfo {
    fn from_raw(device: &DISPLAY_DEVICEW) -> Self {
        Self {
            device_name: wide_array_to_string(&device.DeviceName),
            device_string: wide_array_to_string(&device.DeviceString),
            device_id: wide_array_to_string(&device.DeviceID),
            device_key: wide_array_to_string(&device.DeviceKey),
            is_primary: device.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
            is_attached_to_desktop: device
                .StateFlags
                .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP),
        }
    }
}

fn wide_array_to_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(value.len());

    String::from_utf16_lossy(&value[..end])
}

fn nul_terminated_copy(value: &[u16]) -> Vec<u16> {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(value.len());
    let mut copy = Vec::with_capacity(end + 1);
    copy.extend_from_slice(&value[..end]);
    copy.push(0);
    copy
}
