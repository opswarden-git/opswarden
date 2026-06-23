use tauri_plugin_notification::NotificationExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // N1 capability proof: fire one native OS notification on launch.
            // This links and runs headlessly, but the notification only *appears*
            // on a real desktop session (see client-desktop/README.md). Tying it
            // to a live `incident_assigned` / critical event is a later run.
            if let Err(e) = app
                .notification()
                .builder()
                .title("OpsWarden")
                .body("Desktop shell connected.")
                .show()
            {
                log::warn!("launch notification failed: {e}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
