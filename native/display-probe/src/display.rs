use std::mem::size_of;

use windows::{
    core::PCWSTR,
    Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplaySettingsExW, DEVMODEW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE,
        DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
        ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE,
    },
};

// windows 0.62.2 exposes this Win32 constant from WindowsAndMessaging. Keeping
// the verified value local avoids enabling that otherwise-unused feature.
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x0000_0001;

#[derive(Debug)]
pub struct DisplayAdapter {
    pub index: u32,
    pub info: DisplayDeviceInfo,
    pub device_name_key: Option<Vec<u16>>,
    pub current_mode: Option<DisplayMode>,
    pub available_modes: Vec<EnumeratedDisplayMode>,
    pub monitors: Vec<DisplayMonitor>,
}

#[derive(Debug)]
pub struct DisplayMonitor {
    pub index: u32,
    pub info: DisplayDeviceInfo,
    pub interface_path: MonitorInterfacePath,
}

#[derive(Debug)]
pub enum MonitorInterfacePath {
    Available { value: String, key: Vec<u16> },
    Unavailable,
    InconsistentEnumeration,
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

#[derive(Debug)]
pub struct DisplayMode {
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub refresh_rate: RefreshRate,
}

#[derive(Debug)]
pub struct EnumeratedDisplayMode {
    pub index: u32,
    pub mode: DisplayMode,
}

#[derive(Clone, Copy, Debug)]
pub enum RefreshRate {
    Hertz(u32),
    DriverDefault,
    NotReported,
}

pub fn enumerate_display_adapters() -> Vec<DisplayAdapter> {
    let mut adapters = Vec::new();
    let mut adapter_index = 0_u32;

    loop {
        let Some(raw_adapter) = enum_display_device(None, adapter_index, 0) else {
            break;
        };

        // Keep the exact UTF-16 adapter name for the child-monitor calls. Converting
        // it to a Rust String first could alter malformed UTF-16 code units.
        let adapter_device_name = nul_terminated_copy(&raw_adapter.DeviceName);
        let current_mode = current_display_mode(&adapter_device_name);
        let available_modes = available_display_modes(&adapter_device_name);
        let monitors = enumerate_monitors(&adapter_device_name);

        adapters.push(DisplayAdapter {
            index: adapter_index,
            info: DisplayDeviceInfo::from_raw(&raw_adapter),
            device_name_key: wide_array_to_valid_nonempty_key(&raw_adapter.DeviceName),
            current_mode,
            available_modes,
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
        let Some(raw_monitor) = enum_display_device(Some(adapter_device_name), monitor_index, 0)
        else {
            break;
        };
        let interface_path = query_monitor_interface_path(
            adapter_device_name,
            monitor_index,
            &raw_monitor,
        );

        monitors.push(DisplayMonitor {
            index: monitor_index,
            info: DisplayDeviceInfo::from_raw(&raw_monitor),
            interface_path,
        });

        let Some(next_index) = monitor_index.checked_add(1) else {
            break;
        };
        monitor_index = next_index;
    }

    monitors
}

fn query_monitor_interface_path(
    adapter_device_name: &[u16],
    monitor_index: u32,
    expected_monitor: &DISPLAY_DEVICEW,
) -> MonitorInterfacePath {
    let Some(interface_record) = enum_display_device(
        Some(adapter_device_name),
        monitor_index,
        EDD_GET_DEVICE_INTERFACE_NAME,
    ) else {
        return MonitorInterfacePath::Unavailable;
    };

    if !same_monitor_enumeration_record(expected_monitor, &interface_record) {
        return MonitorInterfacePath::InconsistentEnumeration;
    }

    let Some(key) = wide_array_to_valid_nonempty_key(&interface_record.DeviceID) else {
        return MonitorInterfacePath::Unavailable;
    };
    let value = String::from_utf16(&key)
        .expect("validated monitor interface-path UTF-16 must remain valid");

    MonitorInterfacePath::Available { value, key }
}

fn same_monitor_enumeration_record(
    expected: &DISPLAY_DEVICEW,
    observed: &DISPLAY_DEVICEW,
) -> bool {
    let expected_name = wide_array_to_valid_nonempty_key(&expected.DeviceName);
    let observed_name = wide_array_to_valid_nonempty_key(&observed.DeviceName);

    expected_name.is_some()
        && expected_name == observed_name
        && valid_wide_arrays_equal(&expected.DeviceString, &observed.DeviceString)
        && valid_wide_arrays_equal(&expected.DeviceKey, &observed.DeviceKey)
        && expected.StateFlags == observed.StateFlags
}

fn valid_wide_arrays_equal(left: &[u16], right: &[u16]) -> bool {
    match (wide_array_to_valid_key(left), wide_array_to_valid_key(right)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn current_display_mode(adapter_device_name: &[u16]) -> Option<DisplayMode> {
    assert_eq!(
        adapter_device_name.last(),
        Some(&0),
        "adapter device name must be NUL-terminated"
    );
    let adapter_device_name = PCWSTR::from_raw(adapter_device_name.as_ptr());

    let mut mode = initialized_devmode();

    // SAFETY: `adapter_device_name` points to a NUL-terminated UTF-16 slice that
    // remains alive for the call. `mode` is a valid, aligned, writable DEVMODEW,
    // with `dmSize` initialized to the exact structure size. The function does not
    // retain either pointer. ENUM_CURRENT_SETTINGS with flags 0 only reads the
    // current mode; it does not request or persist a display setting change.
    let succeeded = unsafe {
        EnumDisplaySettingsExW(
            adapter_device_name,
            ENUM_CURRENT_SETTINGS,
            &mut mode,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
    };

    succeeded
        .as_bool()
        .then(|| DisplayMode::from_raw(&mode))
}

fn available_display_modes(adapter_device_name: &[u16]) -> Vec<EnumeratedDisplayMode> {
    assert_eq!(
        adapter_device_name.last(),
        Some(&0),
        "adapter device name must be NUL-terminated"
    );
    let adapter_device_name = PCWSTR::from_raw(adapter_device_name.as_ptr());
    let mut modes = Vec::new();
    let mut mode_index = 0_u32;

    loop {
        let mut mode = initialized_devmode();

        // SAFETY: `adapter_device_name` points to a NUL-terminated UTF-16 slice
        // that remains alive for the call. `mode` is a valid, aligned, writable
        // DEVMODEW with its exact `dmSize` set for every iteration. The function
        // retains neither pointer. A nonnegative mode index with flags 0 enumerates
        // reported modes and does not request or persist a display setting change.
        let succeeded = unsafe {
            EnumDisplaySettingsExW(
                adapter_device_name,
                ENUM_DISPLAY_SETTINGS_MODE(mode_index),
                &mut mode,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
        };

        if !succeeded.as_bool() {
            break;
        }

        modes.push(EnumeratedDisplayMode {
            index: mode_index,
            mode: DisplayMode::from_raw(&mode),
        });

        let Some(next_index) = mode_index.checked_add(1) else {
            break;
        };
        mode_index = next_index;
    }

    modes
}

fn enum_display_device(
    parent_device_name: Option<&[u16]>,
    index: u32,
    flags: u32,
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
    // either pointer after it returns. `flags` is restricted by private callers to
    // zero or EDD_GET_DEVICE_INTERFACE_NAME; both only retrieve device metadata.
    let succeeded = unsafe { EnumDisplayDevicesW(parent_device_name, index, &mut device, flags) };

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

impl DisplayMode {
    fn from_raw(mode: &DEVMODEW) -> Self {
        let width_pixels = mode
            .dmFields
            .contains(DM_PELSWIDTH)
            .then_some(mode.dmPelsWidth)
            .filter(|value| *value > 0);
        let height_pixels = mode
            .dmFields
            .contains(DM_PELSHEIGHT)
            .then_some(mode.dmPelsHeight)
            .filter(|value| *value > 0);
        let refresh_rate = if !mode.dmFields.contains(DM_DISPLAYFREQUENCY) {
            RefreshRate::NotReported
        } else if mode.dmDisplayFrequency <= 1 {
            RefreshRate::DriverDefault
        } else {
            RefreshRate::Hertz(mode.dmDisplayFrequency)
        };

        Self {
            width_pixels,
            height_pixels,
            refresh_rate,
        }
    }
}

fn initialized_devmode() -> DEVMODEW {
    let mut mode = DEVMODEW::default();
    mode.dmSize =
        u16::try_from(size_of::<DEVMODEW>()).expect("DEVMODEW size must fit in a u16");
    mode
}

fn wide_array_to_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(value.len());

    String::from_utf16_lossy(&value[..end])
}

fn wide_array_to_valid_nonempty_key(value: &[u16]) -> Option<Vec<u16>> {
    let key = wide_array_to_valid_key(value)?;
    (!key.is_empty()).then_some(key)
}

fn wide_array_to_valid_key(value: &[u16]) -> Option<Vec<u16>> {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)?;

    String::from_utf16(&value[..end]).ok()?;
    Some(value[..end].to_vec())
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
