#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "windows")]
mod candidate;
#[cfg(target_os = "windows")]
mod ccd;
#[cfg(target_os = "windows")]
mod display;
#[cfg(target_os = "windows")]
mod mapping;
#[cfg(target_os = "windows")]
mod observation;
#[cfg(target_os = "windows")]
mod qualification;

#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_FRIENDLY_NAME_FROM_EDID: u32 = 1 << 0;
#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_FRIENDLY_NAME_FORCED: u32 = 1 << 1;
#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_EDID_IDS_VALID: u32 = 1 << 2;
#[cfg(target_os = "windows")]
const MAX_PRINTED_AVAILABLE_MODE_RECORDS: usize = 8_192;
#[cfg(target_os = "windows")]
const MAX_PRINTED_CANDIDATE_GROUP_INDICES: usize = 8_192;

#[cfg(target_os = "windows")]
fn main() {
    let ccd_query = query_and_print_ccd_snapshot();
    let ccd_snapshot = ccd_query.as_ref().ok();
    println!();

    let inventory = display::enumerate_display_adapters();
    let adapters = &inventory.adapters;
    let verification_snapshot = ccd_snapshot.map(|_| ccd::query_active_display_config());

    println!("GDI Display Inventory");
    print_device_enumeration_status(
        "  AdapterEnumerationStatus",
        inventory.adapter_enumeration_status,
    );
    let mut remaining_available_mode_records =
        MAX_PRINTED_AVAILABLE_MODE_RECORDS;
    if adapters.is_empty() {
        println!("No display adapters found.");
    } else {
        for (adapter_position, adapter) in adapters.iter().enumerate() {
            if adapter_position > 0 {
                println!();
            }

            println!("Adapter {}", adapter.index);
            print_device_info("  ", &adapter.info);
            print_current_mode("  ", &adapter.current_mode);
            print_available_modes(
                "  ",
                &adapter.available_modes,
                adapter.mode_enumeration_status,
                &mut remaining_available_mode_records,
            );
            print_device_enumeration_status(
                "  MonitorEnumerationStatus",
                adapter.monitor_enumeration_status,
            );

            for monitor in &adapter.monitors {
                println!();
                println!("  Monitor {}", monitor.index);
                print_device_info("    ", &monitor.info);
                print_monitor_interface_path("    ", &monitor.interface_path);
            }
        }
    }

    println!();
    let mapping_capture = print_cross_map(
        &ccd_query,
        verification_snapshot.as_ref(),
        &inventory,
    );

    println!();
    let observation_capture = print_current_observations(
        &ccd_query,
        verification_snapshot.as_ref(),
        &mapping_capture,
        adapters,
    );

    println!();
    let candidate_catalog = candidate::build_candidate_catalog(&inventory);
    print_candidate_catalog(&candidate_catalog);

    println!();
    let gdi_environment_markers =
        qualification::collect_gdi_environment_markers(&inventory);
    let qualification = qualification::build_read_only_qualification_with_markers(
        ccd_snapshot,
        &mapping_capture,
        &observation_capture,
        &candidate_catalog,
        gdi_environment_markers,
    );
    print_read_only_qualification(&qualification);
}

#[cfg(target_os = "windows")]
fn query_and_print_ccd_snapshot() -> Result<ccd::CcdSnapshot, ccd::CcdQueryError> {
    println!("CCD Active Configuration");

    let snapshot = match ccd::query_active_display_config() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!("  Error: {error}");
            return Err(error);
        }
    };

    println!("  ActivePaths: {}", snapshot.paths.len());

    for path in &snapshot.paths {
        println!("  Path {}", path.index);
        println!(
            "    Source: adapter={} id={} modeInfoIndex={}",
            format_luid(path.source.adapter_luid),
            path.source.id,
            format_mode_index(path.source.mode_info_index)
        );
        println!(
            "    SourceGdiDeviceName: {}",
            format_optional_log_text(path.source.gdi_device_name.as_deref())
        );
        println!("    SourceStatusFlags: 0x{:08X}", path.source.status_flags);

        if let Some(source_mode) = &path.source_mode {
            println!(
                "    SourceMode: {}x{} at ({}, {}) pixelFormat={}",
                source_mode.width_pixels,
                source_mode.height_pixels,
                source_mode.position_x,
                source_mode.position_y,
                source_mode.pixel_format
            );
        } else {
            println!("    SourceMode: unavailable");
        }

        println!(
            "    Target: adapter={} id={} modeInfoIndex={}",
            format_luid(path.target.adapter_luid),
            path.target.id,
            format_mode_index(path.target.mode_info_index)
        );
        println!(
            "    TargetFriendlyName: {}",
            log_text_or_empty_marker(&path.target.friendly_name)
        );
        println!(
            "    TargetDevicePath: {}",
            format_optional_log_text(path.target.device_path.as_deref())
        );
        println!(
            concat!(
                "    TargetDeviceNameFlags: 0x{:08X} ",
                "(friendlyNameFromEdid={} friendlyNameForced={} edidIdsValid={})"
            ),
            path.target.device_name_flags,
            path.target.device_name_flags & TARGET_NAME_FLAG_FRIENDLY_NAME_FROM_EDID != 0,
            path.target.device_name_flags & TARGET_NAME_FLAG_FRIENDLY_NAME_FORCED != 0,
            path.target.device_name_flags & TARGET_NAME_FLAG_EDID_IDS_VALID != 0
        );
        println!(
            "    TargetMetadataOutputTechnology: {}",
            path.target.metadata_output_technology
        );
        if path.target.device_name_flags & TARGET_NAME_FLAG_EDID_IDS_VALID != 0 {
            println!(
                "    TargetEdidIds: manufacture=0x{:04X} product=0x{:04X}",
                path.target.edid_manufacture_id, path.target.edid_product_code_id
            );
        } else {
            println!("    TargetEdidIds: unavailable (edidIdsValid=false)");
        }
        println!(
            "    TargetConnectorInstance: {}",
            path.target.connector_instance
        );
        println!("    TargetAvailable: {}", path.target.available);
        println!(
            "    TargetPathRefreshRate: {}",
            format_rational(path.target.refresh_rate)
        );
        println!(
            "    OutputTechnology: {}  Rotation: {}  Scaling: {}  ScanLineOrdering: {}",
            path.target.output_technology,
            path.target.rotation,
            path.target.scaling,
            path.target.scan_line_ordering
        );
        println!("    TargetStatusFlags: 0x{:08X}", path.target.status_flags);
        println!("    PathFlags: 0x{:08X}", path.flags);

        if let Some(target_mode) = &path.target_mode {
            println!(
                "    TargetModeActiveSize: {}x{}",
                target_mode.active_width_pixels, target_mode.active_height_pixels
            );
            println!(
                "    TargetModeTotalSize: {}x{}",
                target_mode.total_width_pixels, target_mode.total_height_pixels
            );
            println!(
                "    TargetModeVSync: {}",
                format_rational(target_mode.vertical_sync)
            );
            println!(
                "    TargetModeHSync: {}",
                format_rational(target_mode.horizontal_sync)
            );
            println!("    TargetModePixelRate: {}", target_mode.pixel_rate);
            println!(
                "    TargetModeScanLineOrdering: {}",
                target_mode.scan_line_ordering
            );
        } else {
            println!("    TargetMode: unavailable");
        }
    }

    Ok(snapshot)
}

#[cfg(target_os = "windows")]
fn print_cross_map(
    initial_snapshot: &Result<ccd::CcdSnapshot, ccd::CcdQueryError>,
    verification_snapshot: Option<&Result<ccd::CcdSnapshot, ccd::CcdQueryError>>,
    inventory: &display::DisplayInventory,
) -> qualification::MappingCapture {
    println!("GDI <-> CCD Exact Cross-map");

    let snapshot = match initial_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!(
                "  SnapshotStatus: Unavailable ({:?}: {error})",
                error.failure_class()
            );
            print_empty_mapping_summary(false);
            return qualification::MappingCapture::Unavailable(
                qualification::MappingCaptureFailure::InitialCcdQueryFailed(
                    error.failure_class(),
                ),
            );
        }
    };

    if !display_inventory_is_complete(inventory) {
        println!("  SnapshotStatus: BoundExceeded");
        println!("  GDI adapter/monitor enumeration reached a safety limit.");
        println!("  Exact mapping was not finalized.");
        print_empty_mapping_summary(false);
        return qualification::MappingCapture::Unavailable(
            qualification::MappingCaptureFailure::InventoryBoundExceeded,
        );
    }

    let Some(verification_snapshot) = verification_snapshot else {
        println!("  SnapshotStatus: InternalInconsistency (verification CCD query was not run)");
        print_empty_mapping_summary(false);
        return qualification::MappingCapture::Unavailable(
            qualification::MappingCaptureFailure::InternalInconsistency,
        );
    };
    let verification_snapshot = match verification_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!(
                "  SnapshotStatus: Unavailable ({:?}: {error})",
                error.failure_class()
            );
            println!("  Exact mapping was not finalized.");
            print_empty_mapping_summary(false);
            return qualification::MappingCapture::Unavailable(
                qualification::MappingCaptureFailure::VerificationCcdQueryFailed(
                    error.failure_class(),
                ),
            );
        }
    };

    if !ccd::has_same_mapping_evidence(snapshot, verification_snapshot) {
        println!("  SnapshotStatus: StaleSnapshot");
        println!("  Active CCD mapping evidence changed during GDI enumeration.");
        println!("  Exact mapping was not finalized.");
        print_empty_mapping_summary(true);
        return qualification::MappingCapture::Unavailable(
            qualification::MappingCaptureFailure::StaleSnapshot,
        );
    }

    println!("  SnapshotStatus: SampledStable");
    let cross_map = mapping::cross_map(snapshot, &inventory.adapters);

    for path in &cross_map.paths {
        println!("  Path {}", path.path_index);
        println!("    SourceMatch: {}", path.source_match);
        println!(
            "    SourceAttachedToDesktop: {}",
            format_optional_bool(path.source_attached_to_desktop)
        );
        println!(
            "    SourceEndpointMultiplicity: {}",
            path.source_endpoint_multiplicity
        );
        println!(
            "    SourceEndpointIdentityConsistent: {}",
            path.source_endpoint_identity_consistent
        );
        println!("    SourceInUse: {}", path.source_in_use);
        println!("    TargetMatch: {}", path.target_match);
        println!(
            "    ParentAdapterConsistent: {}",
            format_optional_bool(path.parent_adapter_consistent)
        );
        println!(
            "    TargetAttachedToDesktop: {}",
            format_optional_bool(path.target_attached_to_desktop)
        );
        println!(
            "    OutputTechnologyConsistent: {}",
            path.output_technology_consistent
        );
        println!(
            "    TargetEndpointMultiplicity: {}",
            path.target_endpoint_multiplicity
        );
        println!("    TargetAvailableForSession: {}", path.target_available);
        println!("    TargetInUse: {}", path.target_in_use);
        println!(
            "    TargetForcedAvailability: {}",
            path.target_forced_availability
        );
        println!(
            "    TargetFriendlyNameForced: {}",
            path.target_friendly_name_forced
        );
        println!(
            "    TargetNameHasUnknownFlags: {}",
            path.target_name_has_unknown_flags
        );
        println!("    PathActive: {}", path.path_active);
        println!("    Result: {}", path.classification);
    }

    println!(
        concat!(
            "  Summary: ExactPaths={} UnmappedPaths={} AmbiguousPaths={} ",
            "InconsistentPaths={} Stale=false"
        ),
        cross_map.exact_paths,
        cross_map.unmapped_paths,
        cross_map.ambiguous_paths,
        cross_map.inconsistent_paths
    );

    qualification::MappingCapture::SampledStable(cross_map)
}

#[cfg(target_os = "windows")]
fn print_current_observations(
    initial_snapshot: &Result<ccd::CcdSnapshot, ccd::CcdQueryError>,
    verification_snapshot: Option<&Result<ccd::CcdSnapshot, ccd::CcdQueryError>>,
    mapping_capture: &qualification::MappingCapture,
    adapters: &[display::DisplayAdapter],
) -> qualification::ObservationCapture {
    println!("GDI / CCD Current Observations");

    let snapshot = match initial_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!(
                "  SnapshotStatus: Unavailable ({:?}: {error})",
                error.failure_class()
            );
            print_empty_observation_summary(0, false);
            return qualification::ObservationCapture::Unavailable(
                qualification::ObservationCaptureFailure::InitialCcdQueryFailed(
                    error.failure_class(),
                ),
            );
        }
    };
    let Some(verification_snapshot) = verification_snapshot else {
        println!("  SnapshotStatus: InternalInconsistency (verification CCD query was not run)");
        print_empty_observation_summary(snapshot.paths.len(), false);
        return qualification::ObservationCapture::Unavailable(
            qualification::ObservationCaptureFailure::InternalInconsistency,
        );
    };
    let verification_snapshot = match verification_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!(
                "  SnapshotStatus: Unavailable ({:?}: {error})",
                error.failure_class()
            );
            println!("  Current observations were not finalized.");
            print_empty_observation_summary(snapshot.paths.len(), false);
            return qualification::ObservationCapture::Unavailable(
                qualification::ObservationCaptureFailure::VerificationCcdQueryFailed(
                    error.failure_class(),
                ),
            );
        }
    };

    if !ccd::has_same_current_observation_evidence(snapshot, verification_snapshot) {
        println!("  SnapshotStatus: StaleSnapshot");
        println!("  Active CCD current-observation evidence changed during GDI enumeration.");
        println!("  Current observations were not finalized.");
        print_empty_observation_summary(snapshot.paths.len(), true);
        return qualification::ObservationCapture::Unavailable(
            qualification::ObservationCaptureFailure::StaleSnapshot,
        );
    }

    let Some(cross_map) = mapping_capture.stable_report() else {
        println!("  SnapshotStatus: Unavailable (exact cross-map was not finalized)");
        print_empty_observation_summary(snapshot.paths.len(), false);
        return qualification::ObservationCapture::Unavailable(
            qualification::ObservationCaptureFailure::CrossMapUnavailable,
        );
    };

    println!("  SnapshotStatus: SampledStable");
    println!("  Scope: current resolution/refresh relations only");
    let report = observation::build_current_observations(snapshot, cross_map, adapters);

    for path in &report.paths {
        match path {
            observation::PathObservation::Observed(path) => {
                println!("  Path {}", path.path_index);
                println!(
                    "    Mapping: Adapter {} / Monitor {}",
                    path.adapter_index, path.monitor_index
                );
                println!("    DeviceName: {}", escape_log_text(&path.device_name));
                println!(
                    "    FriendlyName: {}",
                    log_text_or_empty_marker(&path.friendly_label)
                );
                println!("    Rotation: {}", path.rotation);
                println!("    ScalingRaw: {}", path.scaling_raw);
                println!(
                    "    GdiDesktopResolution: {}",
                    format_optional_dimensions(path.gdi_resolution)
                );
                println!(
                    "    CcdSourceResolution: {}",
                    format_optional_dimensions(path.ccd_source_resolution)
                );
                println!(
                    "    RotationAppliedSourceResolution: {}",
                    format_optional_dimensions(path.rotation_applied_source_resolution)
                );
                println!(
                    "    DesktopResolutionRelation: {}",
                    path.desktop_resolution_relation
                );
                println!(
                    "    CcdTargetActiveResolution: {}",
                    format_optional_dimensions(path.ccd_target_active_resolution)
                );
                println!(
                    "    CcdSourceVsTargetActive: {}",
                    path.source_target_resolution_relation
                );
                println!("    GdiRefresh: {}", path.gdi_refresh);
                println!(
                    "    CcdPathRefresh: {}",
                    format_rational(path.ccd_path_refresh)
                );
                println!(
                    "    CcdTargetVSync: {}",
                    format_optional_rational(path.ccd_target_vsync)
                );
                println!(
                    "    GdiVsCcdPathRefresh: {}",
                    path.gdi_vs_ccd_path_refresh
                );
                println!(
                    "    GdiVsCcdTargetVSync: {}",
                    path.gdi_vs_ccd_target_vsync
                );
                println!(
                    "    CcdPathVsTargetVSync: {}",
                    path.ccd_path_vs_target_vsync
                );
                println!("    Result: {}", path.classification);
            }
            observation::PathObservation::Unavailable { path_index, reason } => {
                println!("  Path {path_index}");
                println!("    ObservationUnavailable: {reason}");
                println!("    Result: Unavailable");
            }
        }
    }

    println!(
        concat!(
            "  Summary: ExactPaths={} DistinctPaths={} MismatchPaths={} ",
            "UnavailablePaths={} Stale=false"
        ),
        report.exact_paths,
        report.distinct_paths,
        report.mismatch_paths,
        report.unavailable_paths
    );

    qualification::ObservationCapture::SampledStable(report)
}

#[cfg(target_os = "windows")]
fn print_empty_observation_summary(unavailable_paths: usize, stale: bool) {
    println!(
        concat!(
            "  Summary: ExactPaths=0 DistinctPaths=0 MismatchPaths=0 ",
            "UnavailablePaths={} Stale={}"
        ),
        unavailable_paths, stale
    );
}

#[cfg(target_os = "windows")]
fn print_empty_mapping_summary(stale: bool) {
    println!(
        concat!(
            "  Summary: ExactPaths=0 UnmappedPaths=0 AmbiguousPaths=0 ",
            "InconsistentPaths=0 Stale={}"
        ),
        stale
    );
}

#[cfg(target_os = "windows")]
fn format_luid(luid: ccd::AdapterLuid) -> String {
    format!("0x{:016X}", luid.as_u64())
}

#[cfg(target_os = "windows")]
fn format_mode_index(index: Option<u32>) -> String {
    index
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(target_os = "windows")]
fn format_rational(value: ccd::Rational) -> String {
    if value.numerator == 0 && value.denominator == 0 {
        return "0/0 (unspecified)".to_owned();
    }
    if value.denominator == 0 {
        return format!(
            "{}/{} (invalid denominator)",
            value.numerator, value.denominator
        );
    }
    if value.numerator == 0 {
        return format!("0/{} (non-positive)", value.denominator);
    }

    let decimal = f64::from(value.numerator) / f64::from(value.denominator);
    format!("{}/{} ({decimal:.6} Hz)", value.numerator, value.denominator)
}

#[cfg(target_os = "windows")]
fn format_optional_rational(value: Option<ccd::Rational>) -> String {
    value
        .map(format_rational)
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(target_os = "windows")]
fn format_optional_dimensions(value: Option<observation::Dimensions>) -> String {
    value
        .map(|dimensions| dimensions.to_string())
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(target_os = "windows")]
fn log_text_or_empty_marker(value: &str) -> String {
    if value.is_empty() {
        "(empty)".to_owned()
    } else {
        escape_log_text(value)
    }
}

#[cfg(target_os = "windows")]
fn format_optional_log_text(value: Option<&str>) -> String {
    value
        .map(escape_log_text)
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(target_os = "windows")]
fn escape_log_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        let visually_unsafe = character.is_control()
            || (character.is_whitespace() && character != ' ')
            || matches!(
                character,
                '\u{061C}'
                    | '\u{200B}'..='\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2060}'..='\u{206F}'
                    | '\u{FEFF}'
            );

        if visually_unsafe {
            escaped.extend(character.escape_unicode());
        } else {
            escaped.push(character);
        }
    }

    escaped
}

#[cfg(target_os = "windows")]
fn format_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "not evaluated",
    }
}

#[cfg(target_os = "windows")]
fn print_device_enumeration_status(
    label: &str,
    status: display::DeviceEnumerationStatus,
) {
    match status {
        display::DeviceEnumerationStatus::Complete => println!("{label}: Complete"),
        display::DeviceEnumerationStatus::LimitReached { limit } => {
            println!("{label}: Incomplete (limit {limit} reached)");
        }
    }
}

#[cfg(target_os = "windows")]
fn display_inventory_is_complete(inventory: &display::DisplayInventory) -> bool {
    inventory.adapter_enumeration_status
        == display::DeviceEnumerationStatus::Complete
        && inventory.adapters.iter().all(|adapter| {
            adapter.monitor_enumeration_status
                == display::DeviceEnumerationStatus::Complete
        })
}

#[cfg(target_os = "windows")]
fn print_current_mode(indent: &str, sample: &display::CurrentModeSample) {
    let mode = match sample {
        display::CurrentModeSample::SampledStable(mode) => {
            println!("{indent}CurrentModeSample: SampledStable");
            Some(mode)
        }
        display::CurrentModeSample::Unavailable => {
            println!("{indent}CurrentModeSample: unavailable");
            None
        }
        display::CurrentModeSample::Changed { .. } => {
            println!("{indent}CurrentModeSample: changed during candidate capture");
            None
        }
    };

    let Some(mode) = mode else {
        println!("{indent}CurrentResolution: unavailable");
        println!("{indent}CurrentRefreshRateHz: unavailable");
        return;
    };

    match (mode.width_pixels, mode.height_pixels) {
        (Some(width), Some(height)) if width > 0 && height > 0 => {
            println!("{indent}CurrentResolution: {width}x{height}");
        }
        _ => println!("{indent}CurrentResolution: unavailable"),
    }

    match mode.refresh_rate() {
        display::RefreshRate::Hertz(hertz) => {
            println!("{indent}CurrentRefreshRateHz: {hertz}");
        }
        display::RefreshRate::DriverDefault => {
            println!("{indent}CurrentRefreshRateHz: driver default");
        }
        display::RefreshRate::NotReported => {
            println!("{indent}CurrentRefreshRateHz: unavailable");
        }
    }
}

#[cfg(target_os = "windows")]
fn print_available_modes(
    indent: &str,
    modes: &[display::EnumeratedDisplayMode],
    status: display::ModeEnumerationStatus,
    remaining_records: &mut usize,
) {
    println!("{indent}AvailableModes: {}", modes.len());
    match status {
        display::ModeEnumerationStatus::Complete => {
            println!("{indent}AvailableModesEnumeration: Complete");
        }
        display::ModeEnumerationStatus::EmptyOrUnavailable => {
            println!("{indent}AvailableModesEnumeration: empty or unavailable");
        }
        display::ModeEnumerationStatus::LimitReached { limit } => {
            println!(
                "{indent}AvailableModesEnumeration: Incomplete (limit {limit} reached)"
            );
        }
    }

    let records_to_print = (*remaining_records).min(modes.len());
    for enumerated_mode in modes.iter().take(records_to_print) {
        let mode = &enumerated_mode.mode;
        let resolution = match (mode.width_pixels, mode.height_pixels) {
            (Some(width), Some(height)) if width > 0 && height > 0 => {
                format!("{width}x{height}")
            }
            _ => "resolution unavailable".to_owned(),
        };
        let refresh_rate = match mode.refresh_rate() {
            display::RefreshRate::Hertz(hertz) => format!("{hertz} Hz"),
            display::RefreshRate::DriverDefault => "driver default".to_owned(),
            display::RefreshRate::NotReported => "refresh unavailable".to_owned(),
        };

        println!(
            "{indent}  Mode {}: {resolution} @ {refresh_rate}",
            enumerated_mode.index
        );
    }
    *remaining_records -= records_to_print;
    let omitted_records = modes.len() - records_to_print;
    if omitted_records > 0 {
        println!(
            "{indent}  ModeRecordsOmitted: {omitted_records} (global output limit {})",
            MAX_PRINTED_AVAILABLE_MODE_RECORDS
        );
    }
}

#[cfg(target_os = "windows")]
fn print_candidate_catalog(catalog: &candidate::CandidateCatalog) {
    const MAX_DETAILED_RECORDS: usize = 1_024;

    println!("GDI Mode Candidate Classification");
    println!(
        "  CaptureScope: one bounded normal-mode enumeration (flags=0), bracketed by current-mode samples"
    );
    println!("  CandidateListStability: not claimed (single enumeration)");
    println!("  Mutation: disabled; ProductAllowed=0; SelectionTokens=0");
    println!("  DetailedRecordOutputLimit: {MAX_DETAILED_RECORDS} total");
    println!(
        "  GroupIndexOutputLimit: {} total",
        MAX_PRINTED_CANDIDATE_GROUP_INDICES
    );
    print_device_enumeration_status(
        "  AdapterEnumerationStatus",
        catalog.adapter_enumeration_status,
    );

    let mut remaining_detailed_records = MAX_DETAILED_RECORDS;
    let mut remaining_group_indices = MAX_PRINTED_CANDIDATE_GROUP_INDICES;

    for adapter in &catalog.adapters {
        println!("  Adapter {}", adapter.adapter_index);
        println!(
            "    DeviceName: {}",
            escape_log_text(&adapter.device_name)
        );
        print_device_enumeration_status(
            "    MonitorEnumerationStatus",
            adapter.monitor_enumeration_status,
        );
        match adapter.enumeration_status {
            display::ModeEnumerationStatus::Complete => {
                println!("    EnumerationStatus: Complete");
            }
            display::ModeEnumerationStatus::EmptyOrUnavailable => {
                println!("    EnumerationStatus: EmptyOrUnavailable");
            }
            display::ModeEnumerationStatus::LimitReached { limit } => {
                println!(
                    "    EnumerationStatus: Incomplete (limit {limit} reached)"
                );
            }
        }
        println!("    CurrentTupleStatus: {}", adapter.current_tuple_status);
        println!("    CurrentMembership: {}", adapter.current_membership);
        println!("    CandidateRecords: {}", adapter.candidates.len());
        print_candidate_groups(
            "    ExactDuplicateGroup",
            &adapter.exact_duplicate_groups,
            &mut remaining_group_indices,
        );
        print_candidate_groups(
            "    ProjectionCollisionGroup",
            &adapter.projection_collision_groups,
            &mut remaining_group_indices,
        );

        let records_to_print = remaining_detailed_records.min(adapter.candidates.len());
        for mode in adapter.candidates.iter().take(records_to_print) {
            println!("    Mode {}", mode.provenance.enumeration_index);
            println!(
                "      EnumerationProvenance: adapter={} enumerationIndex={}",
                mode.provenance.adapter_index, mode.provenance.enumeration_index
            );
            println!("      CandidateIdentity: {}", mode.candidate_identity);
            println!("      DisplayLabel: {}", mode.display_label);
            println!(
                concat!(
                    "      ApplyTuple: dmSize={} dmDriverExtra={} dmFields=0x{:08X} ",
                    "position={} orientation={} fixedOutput={} bitsPerPixel={} ",
                    "size={} displayFlags={} frequency={}"
                ),
                mode.public_size_bytes,
                mode.driver_extra_bytes,
                mode.apply_tuple.field_mask,
                format_candidate_position(mode.apply_tuple.position),
                format_candidate_u32(mode.apply_tuple.orientation),
                format_candidate_u32(mode.apply_tuple.fixed_output),
                format_candidate_u32(mode.apply_tuple.bits_per_pixel),
                format_candidate_size(
                    mode.apply_tuple.width_pixels,
                    mode.apply_tuple.height_pixels
                ),
                format_candidate_hex(mode.apply_tuple.display_flags),
                format_candidate_frequency(mode.apply_tuple.display_frequency_hz)
            );
            if mode.tuple_issues.is_empty() {
                println!("      TupleStatus: {}", mode.tuple_status);
            } else {
                println!(
                    "      TupleStatus: {} ({})",
                    mode.tuple_status,
                    format_debug_values(&mode.tuple_issues)
                );
            }
            println!("      ExactDuplicate: {}", mode.exact_duplicate);
            match mode.projection_collision {
                Some(group) => println!(
                    "      ProjectionCollision: Group {} / {} records",
                    group.group_id, group.record_count
                ),
                None => println!("      ProjectionCollision: none"),
            }
            println!("      CurrentRelation: {}", mode.current_relation);
            println!(
                concat!(
                    "      PolicyRelations: position={} orientation={} fixedOutput={} ",
                    "bitsPerPixel={} displayFlags={}"
                ),
                mode.policy_relations.position,
                mode.policy_relations.orientation,
                mode.policy_relations.fixed_output,
                mode.policy_relations.bits_per_pixel,
                mode.policy_relations.display_flags
            );
            println!(
                "      AdvancedColorEvidence: {}",
                mode.advanced_color_evidence
            );
            println!(
                "      ExpectedObservation: {}",
                mode.expected_observation
            );
            println!("      Eligibility: {}", mode.eligibility);
            println!("      SelectionToken: NotIssued (read-only Step 7)");
        }
        remaining_detailed_records -= records_to_print;
        let omitted_records = adapter.candidates.len() - records_to_print;
        if omitted_records > 0 {
            println!(
                "    DetailedCandidateRecordsOmitted: {omitted_records} (summary and groups include all records)"
            );
        }

        print_candidate_summary("    ", adapter.summary);
    }

    println!("  Total");
    print_candidate_summary("    ", catalog.summary);
}

#[cfg(target_os = "windows")]
fn print_read_only_qualification(report: &qualification::ReadOnlyQualification) {
    println!("Read-only Support Assessment");
    println!("  Scope: diagnostic fail-closed precheck only");
    println!("  CellIdentity: NotIssued (read-only Step 8)");
    println!("  SupportFingerprint: NotIssued");
    println!("  CcdQuerySurface: legacy QDC_ONLY_ACTIVE_PATHS");
    println!("  ApprovedCcdSurfaceImplemented: false");
    println!("  MappingCapture: {:?}", report.mapping_status);
    println!("  ObservationCapture: {:?}", report.observation_status);
    println!("  ActivePaths: {:?}", report.active_paths);
    println!("  InventoryComplete: {}", report.inventory_complete);
    println!(
        "  CurrentTupleCaptureComplete: {}",
        report.current_tuple_capture_complete
    );
    println!("  NonExactMappingPaths: {}", report.non_exact_mapping_paths);
    println!("  CloneSourcePaths: {}", report.clone_source_paths);
    println!(
        "  NonExactObservationPaths: {}",
        report.non_exact_observation_paths
    );
    println!(
        "  PortraitRotation: observed={} exact={}",
        report.portrait_rotation_paths, report.portrait_rotation_exact_paths
    );
    println!(
        "  PositiveNonIntegralRefresh: comparisons={} distinct={}",
        report.positive_non_integral_refresh_comparisons,
        report.positive_non_integral_refresh_distinct_comparisons
    );
    println!(
        "  UnqualifiedOutputTechnologyPaths: {}",
        report.unqualified_output_technology_paths
    );
    println!(
        "  CcdNativeEvidenceMarkers: {:?}",
        report.ccd_native_evidence_markers
    );
    println!(
        concat!(
            "  GdiEnvironmentMarkers: attachedAdapters={} attachedMonitors={} ",
            "mirroring={} remoteSdk={} rdpuddSdk={} ",
            "knownUnqualifiedStateFlags={} unknownStateFlags={}"
        ),
        report.gdi_environment_markers.attached_adapter_devices,
        report.gdi_environment_markers.attached_monitor_devices,
        report.gdi_environment_markers.mirroring_driver_devices,
        report.gdi_environment_markers.remote_sdk_devices,
        report.gdi_environment_markers.rdpudd_sdk_devices,
        report
            .gdi_environment_markers
            .known_unqualified_state_flag_devices,
        report.gdi_environment_markers.unknown_state_flag_devices
    );
    println!("  GdiActiveCoverage: {:?}", report.gdi_active_coverage);
    println!("  CandidateVolume: {:?}", report.candidate_volume);
    println!(
        "  ManyCandidateAdapters: {}",
        format_u32_list(&report.many_candidate_adapters)
    );
    println!(
        "  CurrentNotListedAdapters: {}",
        format_u32_list(&report.current_not_listed_adapters)
    );
    println!(
        concat!(
            "  Candidates: records={} labUnqualified={} ",
            "activeAdapterLabUnqualified={} hardExcluded={}"
        ),
        report.candidate_records,
        report.lab_unqualified_candidates,
        report.active_adapter_lab_unqualified_candidates,
        report.hard_excluded_candidates
    );
    print_hard_exclusion_histogram(&report.hard_exclusion_histogram);
    println!("  Invariants: {:?}", report.invariants);
    println!(
        "  InvariantsSatisfied: {}",
        report.invariants.all_satisfied()
    );
    println!("  Blockers:");
    if report.blockers.is_empty() {
        println!("    none (evidence gaps still prevent qualification)");
    } else {
        for blocker in &report.blockers {
            println!("    {blocker:?}");
        }
    }
    println!("  EvidenceGaps:");
    for gap in report.evidence_gaps {
        println!("    {gap:?}");
    }
    println!("  Disposition: {:?}", report.disposition);
    println!("  MutationReadiness: {:?}", report.mutation_readiness);
    println!("  MutationAllowed: false");
    println!("  ProductAllowed: 0");
    println!("  SelectionTokens: 0");
    println!("  G1AGate: {:?}", report.g1a_gate);
    println!("  Phase1AClosure: {:?}", report.phase_1a_closure);
}

#[cfg(target_os = "windows")]
fn print_hard_exclusion_histogram(histogram: &qualification::HardExclusionHistogram) {
    println!(
        "  HardExclusionReasonOccurrences: {}",
        histogram.total_occurrences()
    );
    for (name, count) in [
        ("TupleIncomplete", histogram.tuple_incomplete),
        (
            "AdapterNotAttachedToDesktop",
            histogram.adapter_not_attached_to_desktop,
        ),
        (
            "AdapterEnumerationIncomplete",
            histogram.adapter_enumeration_incomplete,
        ),
        (
            "MonitorEnumerationIncomplete",
            histogram.monitor_enumeration_incomplete,
        ),
        (
            "EnumerationEmptyOrUnavailable",
            histogram.enumeration_empty_or_unavailable,
        ),
        ("EnumerationIncomplete", histogram.enumeration_incomplete),
        ("CurrentUnavailable", histogram.current_unavailable),
        (
            "CurrentChangedDuringCapture",
            histogram.current_changed_during_capture,
        ),
        (
            "CurrentTupleIncomplete",
            histogram.current_tuple_incomplete,
        ),
        ("CurrentNotListed", histogram.current_not_listed),
        (
            "CurrentExactRecordAmbiguous",
            histogram.current_exact_record_ambiguous,
        ),
        ("ExactTupleDuplicate", histogram.exact_tuple_duplicate),
        (
            "DriverDefaultFrequency",
            histogram.driver_default_frequency,
        ),
        (
            "CurrentDriverDefaultFrequency",
            histogram.current_driver_default_frequency,
        ),
        ("BitsPerPixelBelow32", histogram.bits_per_pixel_below_32),
        (
            "CurrentBitsPerPixelBelow32",
            histogram.current_bits_per_pixel_below_32,
        ),
        (
            "KnownButUnsupportedDisplayFlags",
            histogram.known_but_unsupported_display_flags,
        ),
        (
            "CurrentKnownButUnsupportedDisplayFlags",
            histogram.current_known_but_unsupported_display_flags,
        ),
        ("PolicyDifferent", histogram.policy_different),
        (
            "PolicyEvidenceUnavailable",
            histogram.policy_evidence_unavailable,
        ),
    ] {
        if count != 0 {
            println!("    {name}: {count}");
        }
    }
}

#[cfg(target_os = "windows")]
fn format_u32_list(values: &[u32]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(target_os = "windows")]
fn print_candidate_groups(
    label: &str,
    groups: &[candidate::CandidateRecordGroup],
    remaining_indices: &mut usize,
) {
    if groups.is_empty() {
        println!("{label}s: none");
        return;
    }

    for (group_position, group) in groups.iter().enumerate() {
        if *remaining_indices == 0 {
            println!(
                "{label}sOmitted: {} groups (global output limit reached)",
                groups.len() - group_position
            );
            break;
        }

        let indices_to_print = (*remaining_indices).min(group.mode_indices.len());
        let omitted_indices = group.mode_indices.len() - indices_to_print;
        if omitted_indices == 0 {
            println!(
                "{label} {}: Modes {}",
                group.group_id,
                format_candidate_indices(&group.mode_indices)
            );
        } else {
            println!(
                "{label} {}: Modes {} (+{} indices omitted)",
                group.group_id,
                format_candidate_indices(&group.mode_indices[..indices_to_print]),
                omitted_indices
            );
        }
        *remaining_indices -= indices_to_print;
    }
}

#[cfg(target_os = "windows")]
fn print_candidate_summary(indent: &str, summary: candidate::CandidateSummary) {
    println!(
        concat!(
            "{}Summary: Records={} Complete={} Incomplete={} ",
            "ExactDuplicateGroups={} ExactDuplicateRecords={} ",
            "ProjectionCollisionRecords={} LabUnqualified={} HardExcluded={} ",
            "ProductAllowed={} SelectionTokens={}"
        ),
        indent,
        summary.records,
        summary.complete_records,
        summary.incomplete_records,
        summary.exact_duplicate_groups,
        summary.exact_duplicate_records,
        summary.projection_collision_records,
        summary.lab_unqualified_records,
        summary.hard_excluded_records,
        summary.product_allowed_records,
        summary.selection_tokens_issued
    );
}

#[cfg(target_os = "windows")]
fn format_candidate_position(value: Option<display::DisplayPosition>) -> String {
    value
        .map(|position| format!("({},{})", position.x, position.y))
        .unwrap_or_else(|| "not reported".to_owned())
}

#[cfg(target_os = "windows")]
fn format_candidate_u32(value: Option<u32>) -> String {
    value
        .map(|raw| raw.to_string())
        .unwrap_or_else(|| "not reported".to_owned())
}

#[cfg(target_os = "windows")]
fn format_candidate_hex(value: Option<u32>) -> String {
    value
        .map(|raw| format!("0x{raw:08X}"))
        .unwrap_or_else(|| "not reported".to_owned())
}

#[cfg(target_os = "windows")]
fn format_candidate_size(width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(width), Some(height)) => format!("{width}x{height}"),
        _ => "not reported".to_owned(),
    }
}

#[cfg(target_os = "windows")]
fn format_candidate_frequency(value: Option<u32>) -> String {
    match value {
        Some(0) => "driver default (raw 0)".to_owned(),
        Some(1) => "driver default (raw 1)".to_owned(),
        Some(hertz) => format!("{hertz} Hz (raw integer)"),
        None => "not reported".to_owned(),
    }
}

#[cfg(target_os = "windows")]
fn format_candidate_indices(indices: &[u32]) -> String {
    if indices.is_empty() {
        return "none".to_owned();
    }

    indices
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(target_os = "windows")]
fn format_debug_values<T: std::fmt::Debug>(values: &[T]) -> String {
    values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(target_os = "windows")]
fn print_device_info(indent: &str, info: &display::DisplayDeviceInfo) {
    println!("{indent}DeviceName: {}", escape_log_text(&info.device_name));
    println!(
        "{indent}DeviceString: {}",
        escape_log_text(&info.device_string)
    );
    println!("{indent}DeviceID: {}", escape_log_text(&info.device_id));
    println!("{indent}DeviceKey: {}", escape_log_text(&info.device_key));
    println!("{indent}Primary: {}", info.is_primary);
    println!(
        "{indent}AttachedToDesktop: {}",
        info.is_attached_to_desktop
    );
    println!("{indent}StateFlagsRaw: 0x{:08X}", info.state_flags_raw);
    println!(
        "{indent}MirroringDriverMarker: {}",
        info.mirroring_driver_marker
    );
    println!("{indent}RemoteSdkMarker: {}", info.remote_sdk_marker);
    println!("{indent}RdpuddSdkMarker: {}", info.rdpudd_sdk_marker);
}

#[cfg(target_os = "windows")]
fn print_monitor_interface_path(indent: &str, path: &display::MonitorInterfacePath) {
    match path {
        display::MonitorInterfacePath::Available { value, .. } => {
            println!("{indent}DeviceInterfacePath: {}", escape_log_text(value));
        }
        display::MonitorInterfacePath::Unavailable => {
            println!("{indent}DeviceInterfacePath: unavailable");
        }
        display::MonitorInterfacePath::InconsistentEnumeration => {
            println!("{indent}DeviceInterfacePath: inconsistent enumeration");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "display-probe is Windows-only. Build and run it on Windows 10 or Windows 11."
    );
    std::process::exit(1);
}
