use std::{fmt, mem::size_of, ptr};

use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_MODE_INFO_TYPE_TARGET,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_RATIONAL, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    },
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID},
    Graphics::Gdi::DISPLAYCONFIG_PATH_MODE_IDX_INVALID,
};

const MAX_QUERY_ATTEMPTS: usize = 3;
const MAX_PATH_COUNT: u32 = 256;
const MAX_MODE_COUNT: u32 = 1_024;
const TARGET_NAME_FLAG_EDID_IDS_VALID: u32 = 1 << 2;

#[derive(Debug)]
pub struct CcdSnapshot {
    pub paths: Vec<CcdPath>,
}

#[derive(Debug)]
pub struct CcdPath {
    pub index: usize,
    pub source: CcdSource,
    pub target: CcdTarget,
    pub source_mode: Option<CcdSourceMode>,
    pub target_mode: Option<CcdTargetMode>,
    pub flags: u32,
}

#[derive(Debug)]
pub struct CcdSource {
    pub adapter_luid: AdapterLuid,
    pub id: u32,
    pub gdi_device_name: Option<String>,
    pub gdi_device_name_key: Option<Vec<u16>>,
    pub mode_info_index: Option<u32>,
    pub status_flags: u32,
}

#[derive(Debug)]
pub struct CcdTarget {
    pub adapter_luid: AdapterLuid,
    pub id: u32,
    pub friendly_name: String,
    pub device_path: Option<String>,
    pub device_path_key: Option<Vec<u16>>,
    pub device_name_flags: u32,
    pub metadata_output_technology: i32,
    pub edid_manufacture_id: u16,
    pub edid_product_code_id: u16,
    pub connector_instance: u32,
    pub mode_info_index: Option<u32>,
    pub output_technology: i32,
    pub rotation: i32,
    pub scaling: i32,
    pub refresh_rate: Rational,
    pub scan_line_ordering: i32,
    pub available: bool,
    pub status_flags: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CcdSourceMode {
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub pixel_format: i32,
    pub position_x: i32,
    pub position_y: i32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CcdTargetMode {
    pub pixel_rate: u64,
    pub horizontal_sync: Rational,
    pub vertical_sync: Rational,
    pub active_width_pixels: u32,
    pub active_height_pixels: u32,
    pub total_width_pixels: u32,
    pub total_height_pixels: u32,
    pub scan_line_ordering: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterLuid {
    pub low_part: u32,
    pub high_part: i32,
}

impl AdapterLuid {
    pub fn as_u64(self) -> u64 {
        (u64::from(self.high_part as u32) << 32) | u64::from(self.low_part)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rational {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug)]
pub enum CcdQueryError {
    Win32 {
        operation: &'static str,
        code: u32,
    },
    CountLimit {
        path_count: u32,
        mode_count: u32,
    },
    BufferChangedRepeatedly,
    ReturnedCountExceededBuffer,
    ModeIndexOutOfRange {
        path_index: usize,
        role: &'static str,
        mode_index: u32,
        mode_count: usize,
    },
    ModeTypeMismatch {
        path_index: usize,
        role: &'static str,
        mode_index: u32,
        actual_type: i32,
    },
    ModeIdentityMismatch {
        path_index: usize,
        role: &'static str,
        mode_index: u32,
    },
    DeviceInfoHeaderMismatch {
        path_index: usize,
        role: &'static str,
    },
}

impl fmt::Display for CcdQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Win32 { operation, code } => {
                write!(formatter, "{operation} failed with Win32 error {code}")
            }
            Self::CountLimit {
                path_count,
                mode_count,
            } => write!(
                formatter,
                "CCD count limit exceeded: paths={path_count}, modes={mode_count}"
            ),
            Self::BufferChangedRepeatedly => write!(
                formatter,
                "display topology changed during all CCD query attempts"
            ),
            Self::ReturnedCountExceededBuffer => {
                write!(formatter, "QueryDisplayConfig returned an invalid element count")
            }
            Self::ModeIndexOutOfRange {
                path_index,
                role,
                mode_index,
                mode_count,
            } => write!(
                formatter,
                "path {path_index} {role} mode index {mode_index} is outside {mode_count} modes"
            ),
            Self::ModeTypeMismatch {
                path_index,
                role,
                mode_index,
                actual_type,
            } => write!(
                formatter,
                "path {path_index} {role} mode index {mode_index} has type {actual_type}"
            ),
            Self::ModeIdentityMismatch {
                path_index,
                role,
                mode_index,
            } => write!(
                formatter,
                "path {path_index} {role} mode index {mode_index} has a different adapter or ID"
            ),
            Self::DeviceInfoHeaderMismatch { path_index, role } => write!(
                formatter,
                "path {path_index} {role} device-info response header does not match the request"
            ),
        }
    }
}

pub fn query_active_display_config() -> Result<CcdSnapshot, CcdQueryError> {
    let (paths, modes) = query_raw_active_config()?;
    let mut converted_paths = Vec::with_capacity(paths.len());
    let mut source_name_cache = Vec::new();
    let mut target_name_cache = Vec::new();

    for (path_index, path) in paths.iter().enumerate() {
        converted_paths.push(convert_path(
            path_index,
            path,
            &modes,
            &mut source_name_cache,
            &mut target_name_cache,
        )?);
    }

    Ok(CcdSnapshot {
        paths: converted_paths,
    })
}

pub fn has_same_mapping_evidence(left: &CcdSnapshot, right: &CcdSnapshot) -> bool {
    snapshots_have_same_path_multiset(left, right, path_mapping_evidence_equal)
}

pub fn has_same_current_observation_evidence(
    left: &CcdSnapshot,
    right: &CcdSnapshot,
) -> bool {
    snapshots_have_same_path_multiset(left, right, path_current_observation_evidence_equal)
}

fn snapshots_have_same_path_multiset(
    left: &CcdSnapshot,
    right: &CcdSnapshot,
    paths_equal: fn(&CcdPath, &CcdPath) -> bool,
) -> bool {
    if left.paths.len() != right.paths.len() {
        return false;
    }

    let mut matched = vec![false; right.paths.len()];
    for left_path in &left.paths {
        let Some((right_index, _)) = right
            .paths
            .iter()
            .enumerate()
            .find(|(right_index, right_path)| {
                !matched[*right_index] && paths_equal(left_path, right_path)
            })
        else {
            return false;
        };
        matched[right_index] = true;
    }

    true
}

fn path_current_observation_evidence_equal(left: &CcdPath, right: &CcdPath) -> bool {
    path_mapping_evidence_equal(left, right)
        && left.source_mode == right.source_mode
        && left.target_mode == right.target_mode
        && left.target.friendly_name == right.target.friendly_name
        && left.target.rotation == right.target.rotation
        && left.target.scaling == right.target.scaling
        && left.target.refresh_rate == right.target.refresh_rate
        && left.target.scan_line_ordering == right.target.scan_line_ordering
}

fn path_mapping_evidence_equal(left: &CcdPath, right: &CcdPath) -> bool {
    left.source.adapter_luid == right.source.adapter_luid
        && left.source.id == right.source.id
        && left.source.gdi_device_name_key == right.source.gdi_device_name_key
        && left.target.adapter_luid == right.target.adapter_luid
        && left.target.id == right.target.id
        && left.target.device_path_key == right.target.device_path_key
        && left.target.available == right.target.available
        && left.target.output_technology == right.target.output_technology
        && left.target.metadata_output_technology == right.target.metadata_output_technology
        && left.target.device_name_flags == right.target.device_name_flags
        && left.target.connector_instance == right.target.connector_instance
        && target_edid_evidence_equal(&left.target, &right.target)
        && left.source.status_flags == right.source.status_flags
        && left.target.status_flags == right.target.status_flags
        && left.flags == right.flags
}

fn target_edid_evidence_equal(left: &CcdTarget, right: &CcdTarget) -> bool {
    left.device_name_flags & TARGET_NAME_FLAG_EDID_IDS_VALID == 0
        || (left.edid_manufacture_id == right.edid_manufacture_id
            && left.edid_product_code_id == right.edid_product_code_id)
}

fn query_raw_active_config(
) -> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), CcdQueryError> {
    for _ in 0..MAX_QUERY_ATTEMPTS {
        let mut path_count = 0_u32;
        let mut mode_count = 0_u32;

        // SAFETY: Both output pointers refer to valid writable u32 values for the
        // duration of the call. QDC_ONLY_ACTIVE_PATHS is a documented read-only
        // query flag, and the function retains neither pointer.
        let size_result = unsafe {
            GetDisplayConfigBufferSizes(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                &mut mode_count,
            )
        };

        if size_result != ERROR_SUCCESS {
            return Err(CcdQueryError::Win32 {
                operation: "GetDisplayConfigBufferSizes",
                code: size_result.0,
            });
        }

        validate_counts(path_count, mode_count)?;

        if path_count == 0 && mode_count == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut paths = vec![
            DISPLAYCONFIG_PATH_INFO::default();
            usize::try_from(path_count).expect("u32 path count must fit in usize")
        ];
        let mut modes = vec![
            DISPLAYCONFIG_MODE_INFO::default();
            usize::try_from(mode_count).expect("u32 mode count must fit in usize")
        ];
        let path_capacity = path_count;
        let mode_capacity = mode_count;

        let path_pointer = if paths.is_empty() {
            ptr::null_mut()
        } else {
            paths.as_mut_ptr()
        };
        let mode_pointer = if modes.is_empty() {
            ptr::null_mut()
        } else {
            modes.as_mut_ptr()
        };

        // SAFETY: The element counts match the allocated vectors, and the pointers
        // are either NULL for a zero count or valid for writes of that many complete
        // elements. The vectors remain alive and exclusively borrowed for the call.
        // QDC_ONLY_ACTIVE_PATHS is read-only; no topology ID pointer is permitted or
        // provided. The function retains none of the pointers.
        let query_result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                path_pointer,
                &mut mode_count,
                mode_pointer,
                None,
            )
        };

        if query_result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if query_result != ERROR_SUCCESS {
            return Err(CcdQueryError::Win32 {
                operation: "QueryDisplayConfig",
                code: query_result.0,
            });
        }
        if path_count > path_capacity || mode_count > mode_capacity {
            return Err(CcdQueryError::ReturnedCountExceededBuffer);
        }

        paths.truncate(
            usize::try_from(path_count).expect("u32 path count must fit in usize"),
        );
        modes.truncate(
            usize::try_from(mode_count).expect("u32 mode count must fit in usize"),
        );
        return Ok((paths, modes));
    }

    Err(CcdQueryError::BufferChangedRepeatedly)
}

fn validate_counts(path_count: u32, mode_count: u32) -> Result<(), CcdQueryError> {
    if path_count > MAX_PATH_COUNT || mode_count > MAX_MODE_COUNT {
        return Err(CcdQueryError::CountLimit {
            path_count,
            mode_count,
        });
    }

    Ok(())
}

fn convert_path(
    path_index: usize,
    path: &DISPLAYCONFIG_PATH_INFO,
    modes: &[DISPLAYCONFIG_MODE_INFO],
    source_name_cache: &mut Vec<SourceNameCacheEntry>,
    target_name_cache: &mut Vec<TargetNameCacheEntry>,
) -> Result<CcdPath, CcdQueryError> {
    // SAFETY: The query deliberately omits QDC_VIRTUAL_MODE_AWARE, so the
    // documented active union member for each path endpoint is `modeInfoIdx`.
    let source_mode_index = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
    // SAFETY: Same invariant as the source union read immediately above.
    let target_mode_index = unsafe { path.targetInfo.Anonymous.modeInfoIdx };

    let source_mode = resolve_source_mode(
        path_index,
        source_mode_index,
        &path.sourceInfo.adapterId,
        path.sourceInfo.id,
        modes,
    )?;
    let target_mode = resolve_target_mode(
        path_index,
        target_mode_index,
        &path.targetInfo.adapterId,
        path.targetInfo.id,
        modes,
    )?;
    let source_device_name = cached_source_device_name(
        source_name_cache,
        path_index,
        path.sourceInfo.adapterId,
        path.sourceInfo.id,
    )?;
    let target_device_name = cached_target_device_name(
        target_name_cache,
        path_index,
        path.targetInfo.adapterId,
        path.targetInfo.id,
    )?;

    Ok(CcdPath {
        index: path_index,
        source: CcdSource {
            adapter_luid: AdapterLuid::from(path.sourceInfo.adapterId),
            id: path.sourceInfo.id,
            gdi_device_name: source_device_name
                .as_ref()
                .map(|value| value.display.clone()),
            gdi_device_name_key: source_device_name.map(|value| value.key),
            mode_info_index: mode_index(source_mode_index),
            status_flags: path.sourceInfo.statusFlags,
        },
        target: CcdTarget {
            adapter_luid: AdapterLuid::from(path.targetInfo.adapterId),
            id: path.targetInfo.id,
            friendly_name: target_device_name.friendly_name,
            device_path: target_device_name.device_path,
            device_path_key: target_device_name.device_path_key,
            device_name_flags: target_device_name.flags,
            metadata_output_technology: target_device_name.output_technology,
            edid_manufacture_id: target_device_name.edid_manufacture_id,
            edid_product_code_id: target_device_name.edid_product_code_id,
            connector_instance: target_device_name.connector_instance,
            mode_info_index: mode_index(target_mode_index),
            output_technology: path.targetInfo.outputTechnology.0,
            rotation: path.targetInfo.rotation.0,
            scaling: path.targetInfo.scaling.0,
            refresh_rate: Rational::from(path.targetInfo.refreshRate),
            scan_line_ordering: path.targetInfo.scanLineOrdering.0,
            available: path.targetInfo.targetAvailable.as_bool(),
            status_flags: path.targetInfo.statusFlags,
        },
        source_mode,
        target_mode,
        flags: path.flags,
    })
}

#[derive(Clone)]
struct QueriedTargetDeviceName {
    friendly_name: String,
    device_path: Option<String>,
    device_path_key: Option<Vec<u16>>,
    flags: u32,
    output_technology: i32,
    edid_manufacture_id: u16,
    edid_product_code_id: u16,
    connector_instance: u32,
}

#[derive(Clone)]
struct ValidatedWideString {
    display: String,
    key: Vec<u16>,
}

struct SourceNameCacheEntry {
    adapter_id: LUID,
    source_id: u32,
    value: Option<ValidatedWideString>,
}

struct TargetNameCacheEntry {
    adapter_id: LUID,
    target_id: u32,
    value: QueriedTargetDeviceName,
}

fn cached_source_device_name(
    cache: &mut Vec<SourceNameCacheEntry>,
    path_index: usize,
    adapter_id: LUID,
    source_id: u32,
) -> Result<Option<ValidatedWideString>, CcdQueryError> {
    if let Some(entry) = cache
        .iter()
        .find(|entry| entry.adapter_id == adapter_id && entry.source_id == source_id)
    {
        return Ok(entry.value.clone());
    }

    let value = query_source_device_name(path_index, adapter_id, source_id)?;
    cache.push(SourceNameCacheEntry {
        adapter_id,
        source_id,
        value: value.clone(),
    });
    Ok(value)
}

fn cached_target_device_name(
    cache: &mut Vec<TargetNameCacheEntry>,
    path_index: usize,
    adapter_id: LUID,
    target_id: u32,
) -> Result<QueriedTargetDeviceName, CcdQueryError> {
    if let Some(entry) = cache
        .iter()
        .find(|entry| entry.adapter_id == adapter_id && entry.target_id == target_id)
    {
        return Ok(entry.value.clone());
    }

    let value = query_target_device_name(path_index, adapter_id, target_id)?;
    cache.push(TargetNameCacheEntry {
        adapter_id,
        target_id,
        value: value.clone(),
    });
    Ok(value)
}

fn query_source_device_name(
    path_index: usize,
    adapter_id: LUID,
    source_id: u32,
) -> Result<Option<ValidatedWideString>, CcdQueryError> {
    let mut request = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
    request.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        size: u32::try_from(size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>())
            .expect("source device-name packet size must fit in u32"),
        adapterId: adapter_id,
        id: source_id,
    };

    // SAFETY: DISPLAYCONFIG_SOURCE_DEVICE_NAME is repr(C) with `header` as its
    // first field. The raw pointer therefore addresses the full writable packet,
    // whose exact size and GET_SOURCE_NAME discriminator are initialized. The
    // packet remains alive, is uniquely writable, and is not accessed through an
    // alias during the call. The function retains no pointer. This request only
    // retrieves metadata.
    let result = unsafe {
        DisplayConfigGetDeviceInfo(
            ptr::addr_of_mut!(request).cast::<DISPLAYCONFIG_DEVICE_INFO_HEADER>(),
        )
    };
    if result != ERROR_SUCCESS.0 as i32 {
        return Err(CcdQueryError::Win32 {
            operation: "DisplayConfigGetDeviceInfo(GET_SOURCE_NAME)",
            code: result as u32,
        });
    }
    validate_device_info_header(
        path_index,
        "source",
        &request.header,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        u32::try_from(size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>())
            .expect("source device-name packet size must fit in u32"),
        &adapter_id,
        source_id,
    )?;

    Ok(wide_array_to_valid_nonempty_string(
        &request.viewGdiDeviceName,
    ))
}

fn query_target_device_name(
    path_index: usize,
    adapter_id: LUID,
    target_id: u32,
) -> Result<QueriedTargetDeviceName, CcdQueryError> {
    let mut request = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    request.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        size: u32::try_from(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>())
            .expect("target device-name packet size must fit in u32"),
        adapterId: adapter_id,
        id: target_id,
    };

    // SAFETY: DISPLAYCONFIG_TARGET_DEVICE_NAME is repr(C) with `header` first.
    // The pointer addresses the complete writable packet with its exact size and
    // GET_TARGET_NAME discriminator initialized. The packet outlives the call,
    // is uniquely writable, and is not accessed through an alias during the call.
    // The function retains no pointer. This is a read-only metadata request.
    let result = unsafe {
        DisplayConfigGetDeviceInfo(
            ptr::addr_of_mut!(request).cast::<DISPLAYCONFIG_DEVICE_INFO_HEADER>(),
        )
    };
    if result != ERROR_SUCCESS.0 as i32 {
        return Err(CcdQueryError::Win32 {
            operation: "DisplayConfigGetDeviceInfo(GET_TARGET_NAME)",
            code: result as u32,
        });
    }
    validate_device_info_header(
        path_index,
        "target",
        &request.header,
        DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        u32::try_from(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>())
            .expect("target device-name packet size must fit in u32"),
        &adapter_id,
        target_id,
    )?;

    // SAFETY: The complete target-name packet was successfully initialized by
    // DisplayConfigGetDeviceInfo, so reading the raw `value` view of its flags
    // union is valid and avoids interpreting undocumented or unknown bits.
    let flags = unsafe { request.flags.Anonymous.value };

    let device_path = wide_array_to_valid_nonempty_string(&request.monitorDevicePath);

    Ok(QueriedTargetDeviceName {
        friendly_name: wide_array_to_string(&request.monitorFriendlyDeviceName),
        device_path: device_path.as_ref().map(|value| value.display.clone()),
        device_path_key: device_path.map(|value| value.key),
        flags,
        output_technology: request.outputTechnology.0,
        edid_manufacture_id: request.edidManufactureId,
        edid_product_code_id: request.edidProductCodeId,
        connector_instance: request.connectorInstance,
    })
}

fn validate_device_info_header(
    path_index: usize,
    role: &'static str,
    header: &DISPLAYCONFIG_DEVICE_INFO_HEADER,
    expected_type: windows::Win32::Devices::Display::DISPLAYCONFIG_DEVICE_INFO_TYPE,
    expected_size: u32,
    expected_adapter: &LUID,
    expected_id: u32,
) -> Result<(), CcdQueryError> {
    if header.r#type != expected_type
        || header.size != expected_size
        || header.adapterId != *expected_adapter
        || header.id != expected_id
    {
        return Err(CcdQueryError::DeviceInfoHeaderMismatch { path_index, role });
    }

    Ok(())
}

fn resolve_source_mode(
    path_index: usize,
    raw_mode_index: u32,
    expected_adapter: &LUID,
    expected_id: u32,
    modes: &[DISPLAYCONFIG_MODE_INFO],
) -> Result<Option<CcdSourceMode>, CcdQueryError> {
    let Some(mode_index) = mode_index(raw_mode_index) else {
        return Ok(None);
    };
    let mode = mode_at(path_index, "source", mode_index, modes)?;

    if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return Err(CcdQueryError::ModeTypeMismatch {
            path_index,
            role: "source",
            mode_index,
            actual_type: mode.infoType.0,
        });
    }
    validate_mode_identity(
        path_index,
        "source",
        mode_index,
        mode,
        expected_adapter,
        expected_id,
    )?;

    // SAFETY: `infoType` was checked to be SOURCE immediately above, making
    // `sourceMode` the active DISPLAYCONFIG_MODE_INFO union member.
    let source_mode = unsafe { mode.Anonymous.sourceMode };
    Ok(Some(CcdSourceMode {
        width_pixels: source_mode.width,
        height_pixels: source_mode.height,
        pixel_format: source_mode.pixelFormat.0,
        position_x: source_mode.position.x,
        position_y: source_mode.position.y,
    }))
}

fn resolve_target_mode(
    path_index: usize,
    raw_mode_index: u32,
    expected_adapter: &LUID,
    expected_id: u32,
    modes: &[DISPLAYCONFIG_MODE_INFO],
) -> Result<Option<CcdTargetMode>, CcdQueryError> {
    let Some(mode_index) = mode_index(raw_mode_index) else {
        return Ok(None);
    };
    let mode = mode_at(path_index, "target", mode_index, modes)?;

    if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_TARGET {
        return Err(CcdQueryError::ModeTypeMismatch {
            path_index,
            role: "target",
            mode_index,
            actual_type: mode.infoType.0,
        });
    }
    validate_mode_identity(
        path_index,
        "target",
        mode_index,
        mode,
        expected_adapter,
        expected_id,
    )?;

    // SAFETY: `infoType` was checked to be TARGET immediately above, making
    // `targetMode` the active DISPLAYCONFIG_MODE_INFO union member.
    let target_mode = unsafe { mode.Anonymous.targetMode };
    let signal = target_mode.targetVideoSignalInfo;
    Ok(Some(CcdTargetMode {
        pixel_rate: signal.pixelRate,
        horizontal_sync: Rational::from(signal.hSyncFreq),
        vertical_sync: Rational::from(signal.vSyncFreq),
        active_width_pixels: signal.activeSize.cx,
        active_height_pixels: signal.activeSize.cy,
        total_width_pixels: signal.totalSize.cx,
        total_height_pixels: signal.totalSize.cy,
        scan_line_ordering: signal.scanLineOrdering.0,
    }))
}

fn mode_at<'a>(
    path_index: usize,
    role: &'static str,
    mode_index: u32,
    modes: &'a [DISPLAYCONFIG_MODE_INFO],
) -> Result<&'a DISPLAYCONFIG_MODE_INFO, CcdQueryError> {
    modes
        .get(usize::try_from(mode_index).expect("u32 mode index must fit in usize"))
        .ok_or(CcdQueryError::ModeIndexOutOfRange {
            path_index,
            role,
            mode_index,
            mode_count: modes.len(),
        })
}

fn validate_mode_identity(
    path_index: usize,
    role: &'static str,
    mode_index: u32,
    mode: &DISPLAYCONFIG_MODE_INFO,
    expected_adapter: &LUID,
    expected_id: u32,
) -> Result<(), CcdQueryError> {
    if mode.adapterId != *expected_adapter || mode.id != expected_id {
        return Err(CcdQueryError::ModeIdentityMismatch {
            path_index,
            role,
            mode_index,
        });
    }

    Ok(())
}

fn mode_index(raw_index: u32) -> Option<u32> {
    (raw_index != DISPLAYCONFIG_PATH_MODE_IDX_INVALID).then_some(raw_index)
}

fn wide_array_to_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(value.len());

    String::from_utf16_lossy(&value[..end])
}

fn wide_array_to_valid_nonempty_string(value: &[u16]) -> Option<ValidatedWideString> {
    let end = value
        .iter()
        .position(|code_unit| *code_unit == 0)?;

    if end == 0 {
        return None;
    }

    let display = String::from_utf16(&value[..end]).ok()?;
    Some(ValidatedWideString {
        display,
        key: value[..end].to_vec(),
    })
}

impl From<LUID> for AdapterLuid {
    fn from(value: LUID) -> Self {
        Self {
            low_part: value.LowPart,
            high_part: value.HighPart,
        }
    }
}

impl From<DISPLAYCONFIG_RATIONAL> for Rational {
    fn from(value: DISPLAYCONFIG_RATIONAL) -> Self {
        Self {
            numerator: value.Numerator,
            denominator: value.Denominator,
        }
    }
}
