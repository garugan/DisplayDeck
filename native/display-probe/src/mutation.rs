//! Read-only exact-display-cell binding for Gate B. D07 is a separate gate;
//! this module has no display-setting API and cannot apply a captured tuple.

use std::{mem::size_of, ptr, slice};

use windows::{
    core::{PCWSTR, PWSTR},
    Wdk::System::SystemServices::RtlGetVersion,
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::{
                CM_Get_DevNode_PropertyW, CM_Locate_DevNodeW, CM_LOCATE_DEVNODE_NORMAL, CR_SUCCESS,
            },
            Display::{
                DisplayConfigGetDeviceInfo, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
            },
            Properties::{DEVPKEY_Device_DriverVersion, DEVPROPTYPE, DEVPROP_TYPE_STRING},
        },
        Foundation::{ERROR_SUCCESS, LUID},
        System::{
            RemoteDesktop::{
                ProcessIdToSessionId, WTSActive, WTSConnected, WTSDisconnected,
                WTSEnumerateSessionsW, WTSFreeMemory, WTSGetActiveConsoleSessionId,
                WTSQuerySessionInformationW, WTSUserName, WTS_CURRENT_SERVER_HANDLE,
            },
            SystemInformation::{GetProductInfo, OSVERSIONINFOW, PRODUCT_CORE},
        },
        UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION},
    },
};

use crate::{
    candidate::{
        build_candidate_catalog, ApplyTuple, CurrentMembership, CurrentRelation,
        CurrentTupleStatus, ExactDuplicateStatus, FieldRelation, FrequencyLabel, TupleStatus,
    },
    ccd::{self, AdapterLuid, CcdPath, CcdSnapshot, Rational},
    display::{
        self, DeviceEnumerationStatus, DisplayAdapter, DisplayInventory, DisplayMode,
        ModeEnumerationStatus,
    },
    mapping::{self, CrossMap, PathClassification, SourceMatch, TargetMatch},
};

const DEVICE_NAME: &str = r"\\.\DISPLAY1";
const GPU_NAME: &str = "NVIDIA GeForce RTX 4070";
const GPU_PNP_DEVICE_ID: &str = r"PCI\VEN_10DE&DEV_2786&SUBSYS_F3021569&REV_A1\4&341CA995&0&0008";
const GPU_DRIVER_VERSION: &str = "32.0.16.1088";
const MONITOR_NAME: &str = "MSI MAG342CQ";
const CONNECTOR_INSTANCE: u32 = 2;
const DISPLAYPORT_EXTERNAL: i32 = 10;
const WIDTH: u32 = 3440;
const HEIGHT: u32 = 1440;
const CURRENT_HZ: u32 = 144;
const CANDIDATE_HZ: u32 = 60;
const MAX_WTS_SESSIONS: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactCellReadiness {
    Go,
    NoGo,
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExactCellPlan {
    pub readiness: ExactCellReadiness,
    pub blockers: Vec<ExactCellBlocker>,
    active_path_count: Option<usize>,
    binding: Option<ExactBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactCellBlocker {
    CcdQueryFailed,
    StaleTopology,
    InventoryIncomplete,
    ActivePathCountNotOne,
    MappingNotExact,
    TargetIdentityMismatch,
    CurrentModeMismatch,
    CurrentTupleNotUniqueAndComplete,
    CandidateMissingOrAmbiguous,
    RemoteSession,
    NotActiveLocalConsole,
    AdvancedColorEnabled,
    HdrQueryFailed,
    SessionQueryFailed,
    SingleInteractiveUserRequired,
    PlatformCellMismatch,
    GpuIdentityMismatch,
    GpuDriverQueryFailed,
    GpuDriverMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExactBinding {
    candidate_enumeration_index: u32,
    baseline: ApplyTuple,
    candidate: ApplyTuple,
    expected_readback: ExpectedReadback,
    baseline_restore_preflight_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedReadback {
    adapter_luid: AdapterLuid,
    source_id: u32,
    target_id: u32,
    source_name_key: Vec<u16>,
    target_path_key: Vec<u16>,
    width: u32,
    height: u32,
    expected_path_refresh: Rational,
    expected_target_vsync: Rational,
}

impl ExactCellPlan {
    /// Opaque trusted-Rust binding. No frontend type can construct the apply
    /// tuple or its exact expected CCD observation.
    pub fn binding_is_complete(&self) -> bool {
        self.binding.is_some()
    }

    pub fn baseline_restore_preflight_required(&self) -> bool {
        self.binding
            .as_ref()
            .is_some_and(|binding| binding.baseline_restore_preflight_required)
    }

    pub fn active_path_count(&self) -> Option<usize> {
        self.active_path_count
    }

    fn no_go(blocker: ExactCellBlocker) -> Self {
        Self {
            readiness: ExactCellReadiness::NoGo,
            blockers: vec![blocker],
            active_path_count: None,
            binding: None,
        }
    }

    fn sampled_no_go(blocker: ExactCellBlocker, active_path_count: usize) -> Self {
        Self {
            readiness: ExactCellReadiness::NoGo,
            blockers: vec![blocker],
            active_path_count: Some(active_path_count),
            binding: None,
        }
    }
}

/// Performs only enumeration and matching. `Go` means this exact GDI tuple is
/// bound to exact CCD path-refresh and target-vsync 60/1 expectations; it never authorizes mutation.
pub fn assess_exact_cell() -> ExactCellPlan {
    if let Err(blocker) = ensure_platform_cell() {
        return ExactCellPlan::no_go(blocker);
    }
    let console_session = match ensure_local_console() {
        Ok(session) => session,
        Err(blocker) => return ExactCellPlan::no_go(blocker),
    };
    if let Err(blocker) = ensure_single_interactive_user(console_session) {
        return ExactCellPlan::no_go(blocker);
    }
    let capture = match stable_capture() {
        Ok(capture) => capture,
        Err(blocker) => return ExactCellPlan::no_go(blocker),
    };
    let active_path_count = capture.snapshot.paths.len();
    let no_go = |blocker| ExactCellPlan::sampled_no_go(blocker, active_path_count);
    if !inventory_is_complete(&capture.inventory) {
        return no_go(ExactCellBlocker::InventoryIncomplete);
    }
    let (path, mapping) = match exact_single_path(&capture) {
        Ok(value) => value,
        Err(()) => return no_go(ExactCellBlocker::ActivePathCountNotOne),
    };
    if mapping.classification != PathClassification::Exact {
        return no_go(ExactCellBlocker::MappingNotExact);
    }
    let (adapter, monitor) = match exact_adapter_and_monitor(&capture.inventory, mapping) {
        Ok(value) => value,
        Err(()) => return no_go(ExactCellBlocker::MappingNotExact),
    };
    if !is_exact_gpu(adapter) {
        return no_go(ExactCellBlocker::GpuIdentityMismatch);
    }
    match current_driver_version(&adapter.info.device_id) {
        Some(version) if version == GPU_DRIVER_VERSION => {}
        Some(_) => return no_go(ExactCellBlocker::GpuDriverMismatch),
        None => return no_go(ExactCellBlocker::GpuDriverQueryFailed),
    }
    if !is_exact_target(path, adapter, monitor) {
        return no_go(ExactCellBlocker::TargetIdentityMismatch);
    }
    if let Err(blocker) = ensure_hdr_disabled(path) {
        return no_go(blocker);
    }
    let baseline = match complete_tuple(adapter.current_mode.stable_mode()) {
        Ok(tuple) => tuple,
        Err(()) => return no_go(ExactCellBlocker::CurrentTupleNotUniqueAndComplete),
    };
    if !is_mode(&baseline, WIDTH, HEIGHT, CURRENT_HZ)
        || !path_matches_mode(path, WIDTH, HEIGHT, CURRENT_HZ)
    {
        return no_go(ExactCellBlocker::CurrentModeMismatch);
    }

    let catalog = build_candidate_catalog(&capture.inventory);
    let Some(adapter_catalog) = catalog
        .adapters
        .iter()
        .find(|item| item.adapter_index == adapter.index)
    else {
        return no_go(ExactCellBlocker::CurrentTupleNotUniqueAndComplete);
    };
    if adapter_catalog.current_tuple_status != CurrentTupleStatus::Complete
        || !current_membership_allows_stable_baseline(&adapter_catalog.current_membership)
    {
        return no_go(ExactCellBlocker::CurrentTupleNotUniqueAndComplete);
    }
    let candidates = adapter_catalog
        .candidates
        .iter()
        .filter(|candidate| {
            candidate.tuple_status == TupleStatus::Complete
                && candidate.exact_duplicate == ExactDuplicateStatus::Unique
                && candidate.current_relation == CurrentRelation::Different
                && candidate.display_label.width_pixels == Some(WIDTH)
                && candidate.display_label.height_pixels == Some(HEIGHT)
                && candidate.display_label.frequency == FrequencyLabel::Hertz(CANDIDATE_HZ)
                && policy_is_unchanged(candidate.policy_relations)
        })
        .collect::<Vec<_>>();
    let [candidate] = candidates.as_slice() else {
        return no_go(ExactCellBlocker::CandidateMissingOrAmbiguous);
    };
    let (Some(source_name_key), Some(target_path_key)) = (
        path.source.gdi_device_name_key.clone(),
        path.target.device_path_key.clone(),
    ) else {
        return no_go(ExactCellBlocker::MappingNotExact);
    };

    ExactCellPlan {
        readiness: ExactCellReadiness::Go,
        blockers: Vec::new(),
        active_path_count: Some(active_path_count),
        binding: Some(ExactBinding {
            candidate_enumeration_index: candidate.provenance.enumeration_index,
            baseline,
            candidate: candidate.apply_tuple.clone(),
            expected_readback: ExpectedReadback {
                adapter_luid: path.source.adapter_luid,
                source_id: path.source.id,
                target_id: path.target.id,
                source_name_key,
                target_path_key,
                width: WIDTH,
                height: HEIGHT,
                expected_path_refresh: Rational {
                    numerator: CANDIDATE_HZ,
                    denominator: 1,
                },
                expected_target_vsync: Rational {
                    numerator: CANDIDATE_HZ,
                    denominator: 1,
                },
            },
            baseline_restore_preflight_required: true,
        }),
    }
}

struct StableCapture {
    snapshot: CcdSnapshot,
    inventory: DisplayInventory,
    mapping: CrossMap,
}

fn stable_capture() -> Result<StableCapture, ExactCellBlocker> {
    let first = ccd::query_active_display_config().map_err(|_| ExactCellBlocker::CcdQueryFailed)?;
    let inventory = display::enumerate_display_adapters();
    let second =
        ccd::query_active_display_config().map_err(|_| ExactCellBlocker::CcdQueryFailed)?;
    if !ccd::has_same_mapping_evidence(&first, &second)
        || !ccd::has_same_current_observation_evidence(&first, &second)
    {
        return Err(ExactCellBlocker::StaleTopology);
    }
    let mapping = mapping::cross_map(&first, &inventory.adapters);
    Ok(StableCapture {
        snapshot: first,
        inventory,
        mapping,
    })
}

fn inventory_is_complete(inventory: &DisplayInventory) -> bool {
    inventory.adapter_enumeration_status == DeviceEnumerationStatus::Complete
        && !inventory.adapters.is_empty()
        && inventory.adapters.iter().all(|adapter| {
            adapter.monitor_enumeration_status == DeviceEnumerationStatus::Complete
                && adapter.mode_enumeration_status == ModeEnumerationStatus::Complete
        })
}

fn exact_single_path(capture: &StableCapture) -> Result<(&CcdPath, &mapping::PathMapping), ()> {
    let [path] = capture.snapshot.paths.as_slice() else {
        return Err(());
    };
    let [mapping] = capture.mapping.paths.as_slice() else {
        return Err(());
    };
    (mapping.path_index == path.index)
        .then_some((path, mapping))
        .ok_or(())
}

fn exact_adapter_and_monitor<'a>(
    inventory: &'a DisplayInventory,
    mapping: &mapping::PathMapping,
) -> Result<(&'a DisplayAdapter, &'a display::DisplayMonitor), ()> {
    let (SourceMatch::Exact { adapter_index }, TargetMatch::Exact { location }) =
        (&mapping.source_match, &mapping.target_match)
    else {
        return Err(());
    };
    if adapter_index != &location.adapter_index {
        return Err(());
    }
    let adapter = inventory
        .adapters
        .iter()
        .find(|adapter| adapter.index == *adapter_index)
        .ok_or(())?;
    let monitor = adapter
        .monitors
        .iter()
        .find(|monitor| monitor.index == location.monitor_index)
        .ok_or(())?;
    Ok((adapter, monitor))
}

fn is_exact_target(
    path: &CcdPath,
    adapter: &DisplayAdapter,
    monitor: &display::DisplayMonitor,
) -> bool {
    path.source.adapter_luid == path.target.adapter_luid
        && adapter.info.device_name == DEVICE_NAME
        && path.source.gdi_device_name.as_deref() == Some(DEVICE_NAME)
        && monitor.info.device_string == MONITOR_NAME
        && path.target.friendly_name == MONITOR_NAME
        && path.target.connector_instance == CONNECTOR_INSTANCE
        && path.target.output_technology == DISPLAYPORT_EXTERNAL
        && path.target.metadata_output_technology == DISPLAYPORT_EXTERNAL
}

fn is_exact_gpu(adapter: &DisplayAdapter) -> bool {
    adapter.info.device_string == GPU_NAME
        && adapter
            .info
            .device_id
            .eq_ignore_ascii_case(GPU_PNP_DEVICE_ID)
}

fn complete_tuple(mode: Option<&DisplayMode>) -> Result<ApplyTuple, ()> {
    let mode = mode.ok_or(())?;
    let tuple = ApplyTuple::from(mode);
    (mode.public_size_bytes == display::devmode_public_size_bytes()
        && mode.driver_extra_bytes == 0
        && tuple.bits_per_pixel.unwrap_or(0) >= 32
        && tuple.width_pixels.unwrap_or(0) != 0
        && tuple.height_pixels.unwrap_or(0) != 0
        && tuple.display_frequency_hz.unwrap_or(0) > 1)
        .then_some(tuple)
        .ok_or(())
}

fn policy_is_unchanged(policy: crate::candidate::PolicyRelations) -> bool {
    [
        policy.position,
        policy.orientation,
        policy.fixed_output,
        policy.bits_per_pixel,
        policy.display_flags,
    ]
    .into_iter()
    .all(|relation| relation == FieldRelation::Exact)
}

fn current_membership_allows_stable_baseline(membership: &CurrentMembership) -> bool {
    matches!(
        membership,
        CurrentMembership::ListedUnique { .. } | CurrentMembership::NotListedExact { .. }
    )
}

fn is_mode(tuple: &ApplyTuple, width: u32, height: u32, hz: u32) -> bool {
    tuple.width_pixels == Some(width)
        && tuple.height_pixels == Some(height)
        && tuple.display_frequency_hz == Some(hz)
}

fn path_matches_mode(path: &CcdPath, width: u32, height: u32, hz: u32) -> bool {
    path.source_mode
        .as_ref()
        .is_some_and(|mode| mode.width_pixels == width && mode.height_pixels == height)
        && path.target_mode.as_ref().is_some_and(|mode| {
            mode.active_width_pixels == width
                && mode.active_height_pixels == height
                && rational_equals_hertz(mode.vertical_sync, hz)
        })
        && rational_equals_hertz(path.target.refresh_rate, hz)
}

fn rational_equals_hertz(value: Rational, hertz: u32) -> bool {
    value.numerator > 0
        && value.denominator > 0
        && u128::from(value.numerator) == u128::from(hertz) * u128::from(value.denominator)
}

fn ensure_platform_cell() -> Result<(), ExactCellBlocker> {
    if !cfg!(target_arch = "x86_64") {
        return Err(ExactCellBlocker::PlatformCellMismatch);
    }
    let mut version = OSVERSIONINFOW {
        dwOSVersionInfoSize: u32::try_from(size_of::<OSVERSIONINFOW>())
            .expect("OSVERSIONINFOW size fits in u32"),
        ..Default::default()
    };
    // SAFETY: `version` is a writable OSVERSIONINFOW with the documented size.
    if unsafe { RtlGetVersion(&mut version) }.is_err()
        || version.dwMajorVersion != 10
        || version.dwMinorVersion != 0
        || version.dwBuildNumber != 19_045
    {
        return Err(ExactCellBlocker::PlatformCellMismatch);
    }
    let mut product = Default::default();
    // SAFETY: `product` is writable for this documented read-only query.
    if !unsafe { GetProductInfo(10, 0, 0, 0, &mut product) }.as_bool() || product != PRODUCT_CORE {
        return Err(ExactCellBlocker::PlatformCellMismatch);
    }
    Ok(())
}

fn current_driver_version(device_id: &str) -> Option<String> {
    if device_id.is_empty() || device_id.contains('\0') {
        return None;
    }
    let mut device_id = device_id.encode_utf16().collect::<Vec<_>>();
    device_id.push(0);
    let mut device_instance = 0_u32;
    // SAFETY: the device ID is a bounded NUL-terminated UTF-16 string.
    if unsafe {
        CM_Locate_DevNodeW(
            &mut device_instance,
            PCWSTR(device_id.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    } != CR_SUCCESS
    {
        return None;
    }
    let mut value = [0_u16; 64];
    let mut bytes = u32::try_from(size_of_val(&value)).ok()?;
    let mut property_type = DEVPROPTYPE::default();
    // SAFETY: the fixed buffer is writable and its byte size is supplied exactly.
    if unsafe {
        CM_Get_DevNode_PropertyW(
            device_instance,
            &DEVPKEY_Device_DriverVersion,
            &mut property_type,
            Some(value.as_mut_ptr().cast()),
            &mut bytes,
            0,
        )
    } != CR_SUCCESS
        || property_type != DEVPROP_TYPE_STRING
    {
        return None;
    }
    parse_utf16_property(&value, bytes)
}

fn parse_utf16_property(buffer: &[u16], bytes: u32) -> Option<String> {
    let bytes = usize::try_from(bytes).ok()?;
    if bytes < size_of::<u16>() || bytes % size_of::<u16>() != 0 || bytes > size_of_val(buffer) {
        return None;
    }
    let units = bytes / size_of::<u16>();
    let value = buffer.get(..units)?;
    let (&0, body) = value.split_last()? else {
        return None;
    };
    if body.is_empty() || body.contains(&0) {
        return None;
    }
    String::from_utf16(body).ok()
}

fn ensure_local_console() -> Result<u32, ExactCellBlocker> {
    if unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0 {
        return Err(ExactCellBlocker::RemoteSession);
    }
    let mut process_session = 0;
    // SAFETY: `process_session` is writable for the duration of this documented query.
    unsafe { ProcessIdToSessionId(std::process::id(), &mut process_session) }
        .map_err(|_| ExactCellBlocker::NotActiveLocalConsole)?;
    // SAFETY: this documented query has no pointers and returns the active console ID.
    let console_session = unsafe { WTSGetActiveConsoleSessionId() };
    if console_session == u32::MAX || process_session != console_session {
        return Err(ExactCellBlocker::NotActiveLocalConsole);
    }
    Ok(console_session)
}

fn ensure_hdr_disabled(path: &CcdPath) -> Result<(), ExactCellBlocker> {
    let adapter_id = LUID {
        LowPart: path.target.adapter_luid.low_part,
        HighPart: path.target.adapter_luid.high_part,
    };
    let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
    info.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        size: u32::try_from(size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>())
            .expect("advanced-color packet size fits in u32"),
        adapterId: adapter_id,
        id: path.target.id,
    };
    // SAFETY: `info` is a complete writable GET_ADVANCED_COLOR_INFO packet with
    // its documented header initialized. The API retains no pointer.
    let result = unsafe {
        DisplayConfigGetDeviceInfo(
            ptr::addr_of_mut!(info).cast::<DISPLAYCONFIG_DEVICE_INFO_HEADER>(),
        )
    };
    if result != ERROR_SUCCESS.0 as i32
        || info.header.r#type != DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO
        || info.header.size
            != u32::try_from(size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>())
                .expect("advanced-color packet size fits in u32")
        || info.header.adapterId != adapter_id
        || info.header.id != path.target.id
    {
        return Err(ExactCellBlocker::HdrQueryFailed);
    }
    // SAFETY: the successful documented query initializes the union; its raw
    // `value` view contains the documented advancedColorEnabled bit (bit 1).
    let flags = unsafe { info.Anonymous.value };
    if flags & ((1 << 1) | (1 << 2)) != 0 {
        return Err(ExactCellBlocker::AdvancedColorEnabled);
    }
    Ok(())
}

fn ensure_single_interactive_user(console_session: u32) -> Result<(), ExactCellBlocker> {
    let mut sessions = ptr::null_mut();
    let mut count = 0_u32;
    // SAFETY: output pointers remain valid for this documented read-only WTS call.
    unsafe {
        WTSEnumerateSessionsW(
            Some(WTS_CURRENT_SERVER_HANDLE),
            0,
            1,
            &mut sessions,
            &mut count,
        )
    }
    .map_err(|_| ExactCellBlocker::SessionQueryFailed)?;
    let result = interactive_user_sessions(sessions, count, console_session);
    if !sessions.is_null() {
        // SAFETY: WTS allocated this buffer on successful WTSEnumerateSessionsW.
        unsafe { WTSFreeMemory(sessions.cast()) };
    }
    result
}

fn interactive_user_sessions(
    sessions: *mut windows::Win32::System::RemoteDesktop::WTS_SESSION_INFOW,
    count: u32,
    console_session: u32,
) -> Result<(), ExactCellBlocker> {
    if count == 0 || count > MAX_WTS_SESSIONS || sessions.is_null() {
        return Err(ExactCellBlocker::SingleInteractiveUserRequired);
    }
    let sessions = unsafe {
        // SAFETY: successful enumeration returned `count` initialized entries.
        slice::from_raw_parts(
            sessions,
            usize::try_from(count).map_err(|_| ExactCellBlocker::SessionQueryFailed)?,
        )
    };
    let mut users = Vec::new();
    let mut console_is_active = false;
    for session in sessions {
        if session.SessionId == console_session && session.State.0 == WTSActive.0 {
            console_is_active = true;
        }
        if interactive_state(session.State.0) && session_has_nonempty_username(session.SessionId)? {
            users.push(session.SessionId);
        }
    }
    (console_is_active && users.as_slice() == [console_session])
        .then_some(())
        .ok_or(ExactCellBlocker::SingleInteractiveUserRequired)
}

fn interactive_state(state: i32) -> bool {
    state == WTSActive.0 || state == WTSConnected.0 || state == WTSDisconnected.0
}

fn session_has_nonempty_username(session_id: u32) -> Result<bool, ExactCellBlocker> {
    let mut buffer = PWSTR::null();
    let mut bytes = 0_u32;
    // SAFETY: output pointers remain valid for this documented read-only WTS call.
    let query = unsafe {
        WTSQuerySessionInformationW(
            Some(WTS_CURRENT_SERVER_HANDLE),
            session_id,
            WTSUserName,
            &mut buffer,
            &mut bytes,
        )
    };
    if query.is_err() {
        if !buffer.is_null() {
            // SAFETY: WTS returned this buffer even though the operation failed.
            unsafe { WTSFreeMemory(buffer.0.cast()) };
        }
        return Err(ExactCellBlocker::SessionQueryFailed);
    }
    let value = if bytes == 0
        || bytes % u32::try_from(size_of::<u16>()).expect("u16 size fits") != 0
        || buffer.is_null()
    {
        false
    } else {
        let units =
            usize::try_from(bytes / u32::try_from(size_of::<u16>()).expect("u16 size fits"))
                .map_err(|_| ExactCellBlocker::SessionQueryFailed)?;
        let units = unsafe {
            // SAFETY: WTS returned exactly `bytes` readable UTF-16 bytes.
            slice::from_raw_parts(buffer.0, units)
        };
        let Some(end) = units.iter().position(|unit| *unit == 0) else {
            if !buffer.is_null() {
                // SAFETY: WTS allocated this query result buffer on success.
                unsafe { WTSFreeMemory(buffer.0.cast()) };
            }
            return Err(ExactCellBlocker::SessionQueryFailed);
        };
        end != 0
            && units[end..].iter().all(|unit| *unit == 0)
            && String::from_utf16(&units[..end]).is_ok()
    };
    if !buffer.is_null() {
        // SAFETY: WTS allocated this query result buffer on success.
        unsafe { WTSFreeMemory(buffer.0.cast()) };
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_expected_refresh_rejects_5994() {
        assert!(rational_equals_hertz(
            Rational {
                numerator: 60,
                denominator: 1
            },
            60
        ));
        assert!(!rational_equals_hertz(
            Rational {
                numerator: 59_940,
                denominator: 1_000
            },
            60
        ));
    }

    #[test]
    fn binding_never_leaks_from_a_no_go_plan() {
        let plan = ExactCellPlan::no_go(ExactCellBlocker::CandidateMissingOrAmbiguous);
        assert_eq!(plan.readiness, ExactCellReadiness::NoGo);
        assert_eq!(plan.active_path_count(), None);
        assert!(!plan.binding_is_complete());
    }

    #[test]
    fn stable_current_mode_may_be_absent_from_the_normal_list() {
        assert!(current_membership_allows_stable_baseline(
            &CurrentMembership::NotListedExact {
                projection_only_indices: Vec::new()
            }
        ));
        assert!(!current_membership_allows_stable_baseline(
            &CurrentMembership::CurrentUnavailable
        ));
    }

    #[test]
    fn interactive_user_states_are_limited_to_the_documented_three() {
        assert!(interactive_state(WTSActive.0));
        assert!(interactive_state(WTSConnected.0));
        assert!(interactive_state(WTSDisconnected.0));
        assert!(!interactive_state(3));
    }

    #[test]
    fn driver_property_requires_one_terminated_utf16_string() {
        let valid = ['3' as u16, '2' as u16, 0];
        assert_eq!(parse_utf16_property(&valid, 6).as_deref(), Some("32"));
        assert!(parse_utf16_property(&valid, 4).is_none());
        assert!(parse_utf16_property(&[b'3' as u16, 0, b'2' as u16, 0], 8).is_none());
    }
}
