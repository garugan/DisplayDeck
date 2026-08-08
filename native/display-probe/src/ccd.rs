use std::{fmt, ptr};

use windows::Win32::{
    Devices::Display::{
        GetDisplayConfigBufferSizes, QueryDisplayConfig, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_MODE_INFO_TYPE_TARGET,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_RATIONAL, QDC_ONLY_ACTIVE_PATHS,
    },
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID},
    Graphics::Gdi::DISPLAYCONFIG_PATH_MODE_IDX_INVALID,
};

const MAX_QUERY_ATTEMPTS: usize = 3;
const MAX_PATH_COUNT: u32 = 256;
const MAX_MODE_COUNT: u32 = 1_024;

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
    pub mode_info_index: Option<u32>,
    pub status_flags: u32,
}

#[derive(Debug)]
pub struct CcdTarget {
    pub adapter_luid: AdapterLuid,
    pub id: u32,
    pub mode_info_index: Option<u32>,
    pub output_technology: i32,
    pub rotation: i32,
    pub scaling: i32,
    pub refresh_rate: Rational,
    pub scan_line_ordering: i32,
    pub available: bool,
    pub status_flags: u32,
}

#[derive(Debug)]
pub struct CcdSourceMode {
    pub width_pixels: u32,
    pub height_pixels: u32,
    pub pixel_format: i32,
    pub position_x: i32,
    pub position_y: i32,
}

#[derive(Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct AdapterLuid {
    pub low_part: u32,
    pub high_part: i32,
}

impl AdapterLuid {
    pub fn as_u64(self) -> u64 {
        (u64::from(self.high_part as u32) << 32) | u64::from(self.low_part)
    }
}

#[derive(Clone, Copy, Debug)]
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
        }
    }
}

pub fn query_active_display_config() -> Result<CcdSnapshot, CcdQueryError> {
    let (paths, modes) = query_raw_active_config()?;
    let mut converted_paths = Vec::with_capacity(paths.len());

    for (path_index, path) in paths.iter().enumerate() {
        converted_paths.push(convert_path(path_index, path, &modes)?);
    }

    Ok(CcdSnapshot {
        paths: converted_paths,
    })
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

    Ok(CcdPath {
        index: path_index,
        source: CcdSource {
            adapter_luid: AdapterLuid::from(path.sourceInfo.adapterId),
            id: path.sourceInfo.id,
            mode_info_index: mode_index(source_mode_index),
            status_flags: path.sourceInfo.statusFlags,
        },
        target: CcdTarget {
            adapter_luid: AdapterLuid::from(path.targetInfo.adapterId),
            id: path.targetInfo.id,
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
