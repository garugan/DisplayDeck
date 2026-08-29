const COMMANDS: &[&str] = &[
    "get_display_snapshot",
    "begin_display_change",
    "ack_display_change_presentation",
    "confirm_display_change",
    "revert_display_change",
    "get_display_change_status",
    "export_diagnostics",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to build DisplayDeck Tauri context");
}
