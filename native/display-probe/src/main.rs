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

        for monitor in &adapter.monitors {
            println!();
            println!("  Monitor {}", monitor.index);
            print_device_info("    ", &monitor.info);
        }
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
