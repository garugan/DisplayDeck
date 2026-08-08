#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "windows")]
mod display;

#[cfg(target_os = "windows")]
fn main() {
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
