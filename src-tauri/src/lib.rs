mod commands;
mod core;

use commands::*;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .manage(ProjectState::new(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            // Project & Scan commands (F-001)
            create_project,
            scan_directory,
            get_scan_progress,
            // Image processing commands
            get_image_metadata,
            compute_image_hash,
            // File system commands
            select_directory,
            list_directory_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
