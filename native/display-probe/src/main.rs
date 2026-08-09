#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "windows")]
mod ccd;
#[cfg(target_os = "windows")]
mod display;
#[cfg(target_os = "windows")]
mod mapping;
#[cfg(target_os = "windows")]
mod observation;

#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_FRIENDLY_NAME_FROM_EDID: u32 = 1 << 0;
#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_FRIENDLY_NAME_FORCED: u32 = 1 << 1;
#[cfg(target_os = "windows")]
const TARGET_NAME_FLAG_EDID_IDS_VALID: u32 = 1 << 2;

#[cfg(target_os = "windows")]
fn main() {
    let ccd_snapshot = query_and_print_ccd_snapshot();
    println!();

    let adapters = display::enumerate_display_adapters();
    let verification_snapshot = ccd_snapshot
        .as_ref()
        .map(|_| ccd::query_active_display_config());

    if adapters.is_empty() {
        println!("No display adapters found.");
    } else {
        for (adapter_position, adapter) in adapters.iter().enumerate() {
            if adapter_position > 0 {
                println!();
            }

            println!("Adapter {}", adapter.index);
            print_device_info("  ", &adapter.info);
            print_current_mode("  ", adapter.current_mode.as_ref());
            print_available_modes("  ", &adapter.available_modes);

            for monitor in &adapter.monitors {
                println!();
                println!("  Monitor {}", monitor.index);
                print_device_info("    ", &monitor.info);
                print_monitor_interface_path("    ", &monitor.interface_path);
            }
        }
    }

    println!();
    let cross_map = print_cross_map(
        ccd_snapshot.as_ref(),
        verification_snapshot.as_ref(),
        &adapters,
    );

    println!();
    print_current_observations(
        ccd_snapshot.as_ref(),
        verification_snapshot.as_ref(),
        cross_map.as_ref(),
        &adapters,
    );
}

#[cfg(target_os = "windows")]
fn query_and_print_ccd_snapshot() -> Option<ccd::CcdSnapshot> {
    println!("CCD Active Configuration");

    let snapshot = match ccd::query_active_display_config() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!("  Error: {error}");
            return None;
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

    Some(snapshot)
}

#[cfg(target_os = "windows")]
fn print_cross_map(
    snapshot: Option<&ccd::CcdSnapshot>,
    verification_snapshot: Option<&Result<ccd::CcdSnapshot, ccd::CcdQueryError>>,
    adapters: &[display::DisplayAdapter],
) -> Option<mapping::CrossMap> {
    println!("GDI <-> CCD Exact Cross-map");

    let Some(snapshot) = snapshot else {
        println!("  SnapshotStatus: ApiError (initial CCD query failed)");
        print_empty_mapping_summary(false);
        return None;
    };

    let Some(verification_snapshot) = verification_snapshot else {
        println!("  SnapshotStatus: ApiError (verification CCD query was not run)");
        print_empty_mapping_summary(false);
        return None;
    };
    let verification_snapshot = match verification_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!("  SnapshotStatus: ApiError ({error})");
            println!("  Exact mapping was not finalized.");
            print_empty_mapping_summary(false);
            return None;
        }
    };

    if !ccd::has_same_mapping_evidence(snapshot, verification_snapshot) {
        println!("  SnapshotStatus: StaleSnapshot");
        println!("  Active CCD mapping evidence changed during GDI enumeration.");
        println!("  Exact mapping was not finalized.");
        print_empty_mapping_summary(true);
        return None;
    }

    println!("  SnapshotStatus: SampledStable");
    let cross_map = mapping::cross_map(snapshot, adapters);

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

    Some(cross_map)
}

#[cfg(target_os = "windows")]
fn print_current_observations(
    snapshot: Option<&ccd::CcdSnapshot>,
    verification_snapshot: Option<&Result<ccd::CcdSnapshot, ccd::CcdQueryError>>,
    cross_map: Option<&mapping::CrossMap>,
    adapters: &[display::DisplayAdapter],
) {
    println!("GDI / CCD Current Observations");

    let Some(snapshot) = snapshot else {
        println!("  SnapshotStatus: ApiError (initial CCD query failed)");
        print_empty_observation_summary(0, false);
        return;
    };
    let Some(verification_snapshot) = verification_snapshot else {
        println!("  SnapshotStatus: ApiError (verification CCD query was not run)");
        print_empty_observation_summary(snapshot.paths.len(), false);
        return;
    };
    let verification_snapshot = match verification_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!("  SnapshotStatus: ApiError ({error})");
            println!("  Current observations were not finalized.");
            print_empty_observation_summary(snapshot.paths.len(), false);
            return;
        }
    };

    if !ccd::has_same_current_observation_evidence(snapshot, verification_snapshot) {
        println!("  SnapshotStatus: StaleSnapshot");
        println!("  Active CCD current-observation evidence changed during GDI enumeration.");
        println!("  Current observations were not finalized.");
        print_empty_observation_summary(snapshot.paths.len(), true);
        return;
    }

    let Some(cross_map) = cross_map else {
        println!("  SnapshotStatus: Unavailable (exact cross-map was not finalized)");
        print_empty_observation_summary(snapshot.paths.len(), false);
        return;
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
fn print_current_mode(indent: &str, mode: Option<&display::DisplayMode>) {
    let Some(mode) = mode else {
        println!("{indent}CurrentResolution: unavailable");
        println!("{indent}CurrentRefreshRateHz: unavailable");
        return;
    };

    match (mode.width_pixels, mode.height_pixels) {
        (Some(width), Some(height)) => {
            println!("{indent}CurrentResolution: {width}x{height}");
        }
        _ => println!("{indent}CurrentResolution: unavailable"),
    }

    match mode.refresh_rate {
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
fn print_available_modes(indent: &str, modes: &[display::EnumeratedDisplayMode]) {
    println!("{indent}AvailableModes: {}", modes.len());

    for enumerated_mode in modes {
        let mode = &enumerated_mode.mode;
        let resolution = match (mode.width_pixels, mode.height_pixels) {
            (Some(width), Some(height)) => format!("{width}x{height}"),
            _ => "resolution unavailable".to_owned(),
        };
        let refresh_rate = match mode.refresh_rate {
            display::RefreshRate::Hertz(hertz) => format!("{hertz} Hz"),
            display::RefreshRate::DriverDefault => "driver default".to_owned(),
            display::RefreshRate::NotReported => "refresh unavailable".to_owned(),
        };

        println!(
            "{indent}  Mode {}: {resolution} @ {refresh_rate}",
            enumerated_mode.index
        );
    }
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
