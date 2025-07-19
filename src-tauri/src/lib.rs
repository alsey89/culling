// Core library modules
pub mod services;

// Re-export core types for external use
pub use services::{
    ScannerService, HashService, PerceptualService, ScoringService, DatabaseService,
    scanner::{ImageFile, ImageFormat, ScanOptions},
    hash::FileHash,
    perceptual::{PerceptualHash, HashAlgorithm},
    scoring::QualityScore,
    database::{DuplicateGroup, DuplicateType, AppConfig},
};

// Tauri command handlers for frontend integration
#[tauri::command]
async fn scan_directory(path: String, options: ScanOptions) -> Result<Vec<ImageFile>, String> {
    let scanner = ScannerService::new();
    let path_buf = std::path::PathBuf::from(path);
    
    match scanner.scan_directory(path_buf, options).await {
        Ok(mut receiver) => {
            let mut files = Vec::new();
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(file) => files.push(file),
                    Err(e) => log::warn!("Scan error: {}", e),
                }
            }
            Ok(files)
        }
        Err(e) => Err(format!("Failed to scan directory: {}", e)),
    }
}

#[tauri::command]
async fn compute_file_hash(path: String) -> Result<String, String> {
    let hash_service = HashService::new();
    match hash_service.compute_hash(path).await {
        Ok(hash) => Ok(hash.as_str().to_string()),
        Err(e) => Err(format!("Failed to compute hash: {}", e)),
    }
}

#[tauri::command]
async fn compute_perceptual_hash(path: String) -> Result<PerceptualHash, String> {
    let perceptual_service = PerceptualService::new();
    match perceptual_service.compute_perceptual_hash(path).await {
        Ok(hash) => Ok(hash),
        Err(e) => Err(format!("Failed to compute perceptual hash: {}", e)),
    }
}

#[tauri::command]
async fn score_image_quality(path: String) -> Result<QualityScore, String> {
    let scoring_service = ScoringService::new();
    match scoring_service.score_image(path).await {
        Ok(score) => Ok(score),
        Err(e) => Err(format!("Failed to score image: {}", e)),
    }
}

#[tauri::command]
async fn get_app_config() -> Result<AppConfig, String> {
    // This would typically use a global database instance
    // For now, return default config
    Ok(AppConfig::default())
}

#[tauri::command]
async fn save_app_config(config: AppConfig) -> Result<(), String> {
    // This would typically save to a global database instance
    // For now, just log the config
    log::info!("Config saved: {:?}", config);
    Ok(())
}

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
    .invoke_handler(tauri::generate_handler![
        scan_directory,
        compute_file_hash,
        compute_perceptual_hash,
        score_image_quality,
        get_app_config,
        save_app_config
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
