#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "windows")]
mod ccd;
#[cfg(target_os = "windows")]
mod display;

#[cfg(target_os = "windows")]
fn main() {
    print_ccd_snapshot();
    println!();

    let adapters = display::enumerate_display_adapters();

    if adapters.is_empty() {
        println!("No display adapters found.");
        return;
    }

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
        }
    }
}

#[cfg(target_os = "windows")]
fn print_ccd_snapshot() {
    println!("CCD Active Configuration");

    let snapshot = match ccd::query_active_display_config() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            println!("  Error: {error}");
            return;
        }
    };

    println!("  ActivePaths: {}", snapshot.paths.len());

    for path in snapshot.paths {
        println!("  Path {}", path.index);
        println!(
            "    Source: adapter={} id={} modeInfoIndex={}",
            format_luid(path.source.adapter_luid),
            path.source.id,
            format_mode_index(path.source.mode_info_index)
        );
        println!("    SourceStatusFlags: 0x{:08X}", path.source.status_flags);

        if let Some(source_mode) = path.source_mode {
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

        if let Some(target_mode) = path.target_mode {
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
    if value.denominator == 0 {
        return format!("{}/{} (undefined)", value.numerator, value.denominator);
    }

    let decimal = f64::from(value.numerator) / f64::from(value.denominator);
    format!("{}/{} ({decimal:.6} Hz)", value.numerator, value.denominator)
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
    println!("{indent}DeviceName: {}", info.device_name);
    println!("{indent}DeviceString: {}", info.device_string);
    println!("{indent}DeviceID: {}", info.device_id);
    println!("{indent}DeviceKey: {}", info.device_key);
    println!("{indent}Primary: {}", info.is_primary);
    println!(
        "{indent}AttachedToDesktop: {}",
        info.is_attached_to_desktop
    );
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "display-probe is Windows-only. Build and run it on Windows 10 or Windows 11."
    );
    std::process::exit(1);
}
