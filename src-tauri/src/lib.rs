mod commands;
mod core;
mod database;
mod schema;

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

            // Initialize database
            database::connection::init_database()
                .map_err(|e| format!("Failed to initialize database: {}", e))?;

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .manage(ProjectState::new(Mutex::new(None)))
        .manage(commands::ScanState::new(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            // Project & Scan commands (F-001)
            create_project,
            get_recent_projects,
            load_project,
            get_scan_progress,
            scan_project,
            cancel_scan,
            get_enhanced_scan_progress,
            // Project management commands
            get_project_stats,
            rename_project,
            delete_project,
            duplicate_project,
            // Asset commands
            get_project_assets_paginated,
            get_asset_count,
            // Thumbnail commands
            get_thumbnail_path,
            get_thumbnail_data,
            get_project_cache_info,
            generate_thumbnails_background,
            // Image processing commands
            get_image_metadata,
            compute_image_hash,
            // File system commands
            get_default_output_location,
            select_directory,
            list_directory_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
