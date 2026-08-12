use std::mem::size_of;

use windows::{
    core::PCWSTR,
    Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, EnumDisplaySettingsExW, DEVMODEW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER,
        DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICE_RDPUDD, DISPLAY_DEVICE_REMOTE,
        DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFLAGS, DM_DISPLAYFREQUENCY,
        DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
        ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE,
    },
};

// windows 0.62.2 exposes this Win32 constant from WindowsAndMessaging. Keeping
// the verified value local avoids enabling that otherwise-unused feature.
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x0000_0001;

// The Phase 1A execution record bounds the first read-only normal-mode capture
// to indices 0..4095. If every permitted index succeeds, the capture stops at
// the bound and is incomplete; index 4096 is never called.
pub const MAX_ENUMERATED_DISPLAY_MODES: u32 = 4096;
pub const MAX_ENUMERATED_DISPLAY_ADAPTERS: u32 = 32;
pub const MAX_ENUMERATED_MONITORS_PER_ADAPTER: u32 = 32;

pub fn devmode_public_size_bytes() -> u16 {
    u16::try_from(size_of::<DEVMODEW>()).expect("DEVMODEW size must fit in a u16")
}

#[derive(Debug)]
pub struct DisplayInventory {
    pub adapters: Vec<DisplayAdapter>,
    pub adapter_enumeration_status: DeviceEnumerationStatus,
}

#[derive(Debug)]
pub struct DisplayAdapter {
    pub index: u32,
    pub info: DisplayDeviceInfo,
    pub device_name_key: Option<Vec<u16>>,
    pub current_mode: CurrentModeSample,
    pub available_modes: Vec<EnumeratedDisplayMode>,
    pub mode_enumeration_status: ModeEnumerationStatus,
    pub monitors: Vec<DisplayMonitor>,
    pub monitor_enumeration_status: DeviceEnumerationStatus,
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
    pub state_flags_raw: u32,
    pub mirroring_driver_marker: bool,
    pub remote_sdk_marker: bool,
    pub rdpudd_sdk_marker: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayMode {
    pub public_size_bytes: u16,
    pub driver_extra_bytes: u16,
    pub field_mask: u32,
    pub position: Option<DisplayPosition>,
    pub orientation: Option<u32>,
    pub fixed_output: Option<u32>,
    pub bits_per_pixel: Option<u32>,
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub display_flags: Option<u32>,
    pub display_frequency_hz: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DisplayPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub struct EnumeratedDisplayMode {
    pub index: u32,
    pub mode: DisplayMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurrentModeSample {
    SampledStable(DisplayMode),
    Unavailable,
    Changed {
        before: Option<DisplayMode>,
        after: Option<DisplayMode>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModeEnumerationStatus {
    Complete,
    EmptyOrUnavailable,
    LimitReached { limit: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceEnumerationStatus {
    Complete,
    LimitReached { limit: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefreshRate {
    Hertz(u32),
    DriverDefault,
    NotReported,
}

pub fn enumerate_display_adapters() -> DisplayInventory {
    let mut adapters = Vec::new();
    let mut adapter_index = 0_u32;

    let adapter_enumeration_status = loop {
        if adapter_index >= MAX_ENUMERATED_DISPLAY_ADAPTERS {
            break DeviceEnumerationStatus::LimitReached {
                limit: MAX_ENUMERATED_DISPLAY_ADAPTERS,
            };
        }

        let Some(raw_adapter) = enum_display_device(None, adapter_index, 0) else {
            break DeviceEnumerationStatus::Complete;
        };

        // Keep the exact UTF-16 adapter name for the child-monitor calls. Converting
        // it to a Rust String first could alter malformed UTF-16 code units.
        let adapter_device_name = nul_terminated_copy(&raw_adapter.DeviceName);
        let current_mode_before = current_display_mode(&adapter_device_name);
        let (available_modes, mode_enumeration_status) =
            available_display_modes(&adapter_device_name);
        let current_mode_after = current_display_mode(&adapter_device_name);
        let current_mode = CurrentModeSample::from_samples(
            current_mode_before,
            current_mode_after,
        );
        let (monitors, monitor_enumeration_status) =
            enumerate_monitors(&adapter_device_name);

        adapters.push(DisplayAdapter {
            index: adapter_index,
            info: DisplayDeviceInfo::from_raw(&raw_adapter),
            device_name_key: wide_array_to_valid_nonempty_key(&raw_adapter.DeviceName),
            current_mode,
            available_modes,
            mode_enumeration_status,
            monitors,
            monitor_enumeration_status,
        });

        let Some(next_index) = adapter_index.checked_add(1) else {
            break DeviceEnumerationStatus::LimitReached {
                limit: MAX_ENUMERATED_DISPLAY_ADAPTERS,
            };
        };
        adapter_index = next_index;
    };

    DisplayInventory {
        adapters,
        adapter_enumeration_status,
    }
}

fn enumerate_monitors(
    adapter_device_name: &[u16],
) -> (Vec<DisplayMonitor>, DeviceEnumerationStatus) {
    let mut monitors = Vec::new();
    let mut monitor_index = 0_u32;

    loop {
        if monitor_index >= MAX_ENUMERATED_MONITORS_PER_ADAPTER {
            return (
                monitors,
                DeviceEnumerationStatus::LimitReached {
                    limit: MAX_ENUMERATED_MONITORS_PER_ADAPTER,
                },
            );
        }

        let Some(raw_monitor) = enum_display_device(Some(adapter_device_name), monitor_index, 0)
        else {
            return (monitors, DeviceEnumerationStatus::Complete);
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
            return (
                monitors,
                DeviceEnumerationStatus::LimitReached {
                    limit: MAX_ENUMERATED_MONITORS_PER_ADAPTER,
                },
            );
        };
        monitor_index = next_index;
    }
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

fn available_display_modes(
    adapter_device_name: &[u16],
) -> (Vec<EnumeratedDisplayMode>, ModeEnumerationStatus) {
    assert_eq!(
        adapter_device_name.last(),
        Some(&0),
        "adapter device name must be NUL-terminated"
    );
    let adapter_device_name = PCWSTR::from_raw(adapter_device_name.as_ptr());
    let mut modes = Vec::new();
    let mut mode_index = 0_u32;

    loop {
        if mode_index >= MAX_ENUMERATED_DISPLAY_MODES {
            return (
                modes,
                ModeEnumerationStatus::LimitReached {
                    limit: MAX_ENUMERATED_DISPLAY_MODES,
                },
            );
        }

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
            let status = if modes.is_empty() {
                ModeEnumerationStatus::EmptyOrUnavailable
            } else {
                ModeEnumerationStatus::Complete
            };
            return (modes, status);
        }

        modes.push(EnumeratedDisplayMode {
            index: mode_index,
            mode: DisplayMode::from_raw(&mode),
        });

        let Some(next_index) = mode_index.checked_add(1) else {
            return (
                modes,
                ModeEnumerationStatus::LimitReached {
                    limit: MAX_ENUMERATED_DISPLAY_MODES,
                },
            );
        };
        mode_index = next_index;
    }
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
            state_flags_raw: device.StateFlags.0,
            // These are positive SDK markers only. Their absence does not prove
            // that the caller is the sole local console session.
            mirroring_driver_marker: device
                .StateFlags
                .contains(DISPLAY_DEVICE_MIRRORING_DRIVER),
            remote_sdk_marker: device.StateFlags.contains(DISPLAY_DEVICE_REMOTE),
            rdpudd_sdk_marker: device.StateFlags.contains(DISPLAY_DEVICE_RDPUDD),
        }
    }
}

impl DisplayMode {
    fn from_raw(mode: &DEVMODEW) -> Self {
        let has_display_union_field = mode.dmFields.contains(DM_POSITION)
            || mode.dmFields.contains(DM_DISPLAYORIENTATION)
            || mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT);
        let display_fields = has_display_union_field.then(|| {
            // SAFETY: EnumDisplaySettingsExW was called for a display device and
            // returned this initialized DEVMODEW. At least one corresponding
            // display dmFields bit is set, so the display member of Anonymous1 is
            // the documented active interpretation. Its fields contain only
            // integer/wrapper values for which every bit pattern is valid. The
            // value is copied while `mode` remains borrowed and is never retained.
            unsafe { mode.Anonymous1.Anonymous2 }
        });
        let display_flags = mode.dmFields.contains(DM_DISPLAYFLAGS).then(|| {
            // SAFETY: DM_DISPLAYFLAGS marks the display-flags member of Anonymous2
            // as valid for this returned display DEVMODEW. The member is a u32, so
            // every bit pattern is valid, and it is copied during this borrow.
            unsafe { mode.Anonymous2.dmDisplayFlags }
        });

        Self {
            public_size_bytes: mode.dmSize,
            driver_extra_bytes: mode.dmDriverExtra,
            field_mask: mode.dmFields.0,
            position: mode.dmFields.contains(DM_POSITION).then(|| {
                let position = display_fields
                    .expect("display union must be available when DM_POSITION is set")
                    .dmPosition;
                DisplayPosition {
                    x: position.x,
                    y: position.y,
                }
            }),
            orientation: mode.dmFields.contains(DM_DISPLAYORIENTATION).then(|| {
                display_fields
                    .expect("display union must be available when orientation is set")
                    .dmDisplayOrientation
                    .0
            }),
            fixed_output: mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT).then(|| {
                display_fields
                    .expect("display union must be available when fixed output is set")
                    .dmDisplayFixedOutput
                    .0
            }),
            bits_per_pixel: mode
                .dmFields
                .contains(DM_BITSPERPEL)
                .then_some(mode.dmBitsPerPel),
            width_pixels: mode
                .dmFields
                .contains(DM_PELSWIDTH)
                .then_some(mode.dmPelsWidth),
            height_pixels: mode
                .dmFields
                .contains(DM_PELSHEIGHT)
                .then_some(mode.dmPelsHeight),
            display_flags,
            display_frequency_hz: mode
                .dmFields
                .contains(DM_DISPLAYFREQUENCY)
                .then_some(mode.dmDisplayFrequency),
        }
    }

    pub fn refresh_rate(&self) -> RefreshRate {
        match self.display_frequency_hz {
            Some(hertz) if hertz > 1 => RefreshRate::Hertz(hertz),
            Some(_) => RefreshRate::DriverDefault,
            None => RefreshRate::NotReported,
        }
    }
}

impl CurrentModeSample {
    fn from_samples(before: Option<DisplayMode>, after: Option<DisplayMode>) -> Self {
        match (before, after) {
            (Some(before), Some(after)) if before == after => Self::SampledStable(before),
            (None, None) => Self::Unavailable,
            (before, after) => Self::Changed { before, after },
        }
    }

    pub fn stable_mode(&self) -> Option<&DisplayMode> {
        match self {
            Self::SampledStable(mode) => Some(mode),
            Self::Unavailable | Self::Changed { .. } => None,
        }
    }
}

fn initialized_devmode() -> DEVMODEW {
    let mut mode = DEVMODEW::default();
    mode.dmSize = devmode_public_size_bytes();
    mode.dmDriverExtra = 0;
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
