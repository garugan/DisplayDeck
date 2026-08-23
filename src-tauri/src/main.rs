fn main() {
    let webview_runtime = tauri::webview_version();
    if let Err(error) = displaydeck_app_lib::run() {
        let message = format!(
            "DisplayDeck startup failed: {error}\nDebug: {error:?}\nWebView runtime: {webview_runtime:?}\n"
        );
        eprint!("{message}");
        let directory = std::env::temp_dir().join("DisplayDeck-Stage1");
        if std::fs::create_dir_all(&directory).is_ok() {
            let _ = std::fs::write(directory.join("startup-error.txt"), message);
        }
        std::process::exit(1);
    }
}
