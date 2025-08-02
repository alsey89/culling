use crate::core::{
    image::{ImageHash, ImageMetadata},
    project::{Project, ProjectConfig, ScanProgress},
    scanner::{ScanProgress as EnhancedScanProgress, ScannerService, ScanPhase},
};
use crate::database::{
    connection::get_connection,
    models::{NewProject, Project as DbProject, ScanStatus},
};
use chrono::Utc;
use diesel::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

// Global state for the current project
pub type ProjectState = Arc<Mutex<Option<Project>>>;

// Global state for scan operations
pub type ScanState = Arc<Mutex<Option<Arc<AtomicBool>>>>;

#[tauri::command]
pub async fn create_project(
    source_dir: String,
    output_dir: String,
    project_name: String,
    state: State<'_, ProjectState>,
) -> Result<ProjectConfig, String> {
    use crate::schema::projects;

    // Create the in-memory project first
    let project = Project::new(source_dir.clone(), output_dir.clone(), project_name.clone())
        .map_err(|e| e.to_string())?;

    let config = project.config.clone();

    // Generate a unique project ID
    let project_id = format!("prj_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    // Create database record
    let new_project = NewProject {
        id: project_id.clone(),
        name: project_name,
        source_path: source_dir,
        output_path: output_dir,
        exclude_patterns: "[]".to_string(), // Default empty array
        file_types: r#"["jpg","jpeg","png","heic","tiff","webp","cr2","nef","arw"]"#.to_string(),
        scan_status: String::from(ScanStatus::NotStarted),
        created_at: now.clone(),
        updated_at: now,
    };

    // Insert into database
    let mut conn = get_connection().map_err(|e| e.to_string())?;
    diesel::insert_into(projects::table)
        .values(&new_project)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to save project: {}", e))?;

    // Store the project in global state
    let mut project_state = state.lock().await;
    *project_state = Some(project);

    Ok(config)
}

#[tauri::command]
pub async fn scan_directory(state: State<'_, ProjectState>) -> Result<(), String> {
    let mut project_state = state.lock().await;

    if let Some(ref mut project) = project_state.as_mut() {
        project.scan_images().await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No project loaded".to_string())
    }
}

#[tauri::command]
pub async fn scan_project_enhanced(
    project_id: String,
    app_handle: AppHandle,
    scan_state: State<'_, ScanState>,
) -> Result<(), String> {
    use crate::schema::projects::dsl::*;
    use std::path::PathBuf;

    // Get project from database
    let mut conn = get_connection().map_err(|e| e.to_string())?;
    let db_project = projects
        .filter(id.eq(&project_id))
        .first::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to load project: {}", e))?;

    // Parse configuration
    let parsed_exclude_patterns: Vec<String> =
        serde_json::from_str(&db_project.exclude_patterns).unwrap_or_default();
    let parsed_file_types: Vec<String> = serde_json::from_str(&db_project.file_types)
        .unwrap_or_else(|_| vec!["jpg".to_string(), "jpeg".to_string(), "png".to_string()]);

    // Update scan status to in progress
    diesel::update(projects.filter(id.eq(&project_id)))
        .set(scan_status.eq(String::from(ScanStatus::InProgress)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update scan status: {}", e))?;

    // Set up progress channel
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<EnhancedScanProgress>();

    // Create scanner service with progress sender
    let scanner = ScannerService::new().with_progress_sender(progress_tx);
    let cancellation_token = scanner.get_cancellation_token();

    // Store cancellation token in global state
    {
        let mut scan_state_guard = scan_state.lock().await;
        *scan_state_guard = Some(cancellation_token.clone());
    }

    // Spawn task to forward progress to frontend and handle real-time asset insertion
    let app_handle_clone = app_handle.clone();
    let project_id_clone = project_id.clone();
    let progress_forwarder = tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            // Emit progress event to frontend
            let _ = app_handle_clone.emit("scan-progress", &progress);

            // When quick scan is complete, emit event so UI can show assets
            if progress.quick_scan_complete && progress.phase == ScanPhase::QuickScan {
                let _ = app_handle_clone.emit("quick-scan-complete", &project_id_clone);
            }
        }
    });

    // Perform the scan with real-time database updates
    let scan_result = scan_with_realtime_updates(
        &scanner,
        &project_id,
        &[PathBuf::from(&db_project.source_path)],
        &parsed_file_types,
        &parsed_exclude_patterns,
    )
    .await;

    // Clean up scan state
    {
        let mut scan_state_guard = scan_state.lock().await;
        *scan_state_guard = None;
    }

    // Wait for progress forwarder to finish
    progress_forwarder.abort();

    match scan_result {
        Ok(_) => {
            // Update scan status to completed
            let mut conn = get_connection().map_err(|e| e.to_string())?;
            diesel::update(projects.filter(id.eq(&project_id)))
                .set(scan_status.eq(String::from(ScanStatus::Completed)))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update scan status: {}", e))?;

            // Emit completion event
            let _ = app_handle.emit("scan-complete", &project_id);

            Ok(())
        }
        Err(crate::core::scanner::ScanError::Cancelled) => {
            // Update scan status to cancelled
            let mut conn = get_connection().map_err(|e| e.to_string())?;
            diesel::update(projects.filter(id.eq(&project_id)))
                .set(scan_status.eq(String::from(ScanStatus::Cancelled)))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update scan status: {}", e))?;

            // Emit cancellation event
            let _ = app_handle.emit("scan-cancelled", &project_id);

            Err("Scan was cancelled".to_string())
        }
        Err(e) => {
            // Update scan status to failed
            let mut conn = get_connection().map_err(|e| e.to_string())?;
            diesel::update(projects.filter(id.eq(&project_id)))
                .set(scan_status.eq(String::from(ScanStatus::Failed(e.to_string()))))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update scan status: {}", e))?;

            // Emit error event
            let _ = app_handle.emit("scan-error", format!("Scan failed: {}", e));

            Err(format!("Scan failed: {}", e))
        }
    }
}

/// Enhanced scan function that inserts assets in real-time during the two-phase process
async fn scan_with_realtime_updates(
    scanner: &ScannerService,
    project_id: &str,
    paths: &[PathBuf],
    file_types: &[String],
    exclude_patterns: &[String],
) -> Result<(), crate::core::scanner::ScanError> {
    // Use the existing scan_paths method but with enhanced database integration
    let assets = scanner
        .scan_paths(project_id, paths, file_types, exclude_patterns)
        .await?;

    // Insert all assets to database after scanning is complete
    use crate::database::models::NewAsset;
    use crate::schema::assets;
    use diesel::prelude::*;

    let mut conn = get_connection().map_err(|e| {
        crate::core::scanner::ScanError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Database connection failed: {}", e),
        ))
    })?;

    let new_assets: Vec<NewAsset> = assets
        .iter()
        .map(|asset| NewAsset {
            id: asset.id.clone(),
            project_id: asset.project_id.clone(),
            path: asset.path.clone(),
            thumbnail_path: asset.thumbnail_path.clone(),
            hash: asset.hash.clone(),
            perceptual_hash: asset.perceptual_hash.clone(),
            size: asset.size,
            width: asset.width,
            height: asset.height,
            exif_data: asset.exif_data.clone(),
            created_at: asset.created_at.clone(),
            updated_at: asset.updated_at.clone(),
        })
        .collect();

    diesel::insert_into(assets::table)
        .values(&new_assets)
        .execute(&mut conn)
        .map_err(|e| {
            crate::core::scanner::ScanError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to insert assets: {}", e),
            ))
        })?;

    Ok(())
}

#[tauri::command]
pub async fn cancel_scan(scan_state: State<'_, ScanState>) -> Result<(), String> {
    let scan_state_guard = scan_state.lock().await;

    if let Some(cancellation_token) = scan_state_guard.as_ref() {
        cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    } else {
        Err("No active scan to cancel".to_string())
    }
}

#[tauri::command]
pub async fn get_enhanced_scan_progress(
    _project_id: String,
) -> Result<Option<EnhancedScanProgress>, String> {
    // This would typically be stored in a global state or cache
    // For now, we'll return None as the progress is sent via events
    Ok(None)
}

#[tauri::command]
pub async fn get_scan_progress(
    project_state: tauri::State<'_, ProjectState>,
) -> Result<ScanProgress, String> {
    let project_guard = project_state.lock().await;
    if let Some(project) = &*project_guard {
        Ok(project.get_scan_progress().await)
    } else {
        Err("No project found".to_string())
    }
}

#[tauri::command]
pub async fn get_image_metadata(path: String) -> Result<ImageMetadata, String> {
    ImageMetadata::from_path(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn compute_image_hash(path: String) -> Result<ImageHash, String> {
    ImageHash::compute(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_output_location() -> Result<String, String> {
    use dirs::document_dir;

    match document_dir() {
        Some(mut documents_path) => {
            documents_path.push("Cullrs");
            Ok(documents_path.to_string_lossy().to_string())
        }
        None => {
            // Fallback to home directory if documents directory is not available
            match dirs::home_dir() {
                Some(mut home_path) => {
                    home_path.push("Documents");
                    home_path.push("Cullrs");
                    Ok(home_path.to_string_lossy().to_string())
                }
                None => {
                    // Last resort: use current directory
                    use std::env;
                    let mut current_dir = env::current_dir().map_err(|e| e.to_string())?;
                    current_dir.push("Cullrs");
                    Ok(current_dir.to_string_lossy().to_string())
                }
            }
        }
    }
}

#[tauri::command]
pub async fn select_directory() -> Result<Option<String>, String> {
    // Directory selection will be implemented later with proper Tauri dialog
    // For now, return an error to indicate it's not implemented
    Err("Directory selection not implemented yet".to_string())
}

#[tauri::command]
pub async fn list_directory_images(path: String) -> Result<Vec<String>, String> {
    use std::fs;

    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut image_files = Vec::new();

    let image_extensions = vec![
        "jpg", "jpeg", "png", "tiff", "tif", "bmp", "gif", "webp", "raw", "cr2", "nef", "arw",
    ];

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if let Some(ext_str) = extension.to_str() {
                    if image_extensions.contains(&ext_str.to_lowercase().as_str()) {
                        image_files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    Ok(image_files)
}

#[tauri::command]
pub async fn get_recent_projects() -> Result<Vec<DbProject>, String> {
    use crate::schema::projects::dsl::*;

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    let recent_projects = projects
        .order(created_at.desc())
        .limit(10)
        .load::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to load recent projects: {}", e))?;

    Ok(recent_projects)
}

#[tauri::command]
pub async fn load_project(
    project_id: String,
    state: State<'_, ProjectState>,
) -> Result<DbProject, String> {
    use crate::schema::projects::dsl::*;

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    let db_project = projects
        .filter(id.eq(&project_id))
        .first::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to load project: {}", e))?;

    // Parse the exclude patterns and file types from JSON
    let _exclude_patterns: Vec<String> =
        serde_json::from_str(&db_project.exclude_patterns).unwrap_or_default();
    let _file_types: Vec<String> = serde_json::from_str(&db_project.file_types)
        .unwrap_or_else(|_| vec!["jpg".to_string(), "jpeg".to_string(), "png".to_string()]);

    // Create in-memory project from database record
    let project = Project::new(
        db_project.source_path.clone(),
        db_project.output_path.clone(),
        db_project.name.clone(),
    )
    .map_err(|e| e.to_string())?;

    // Store the project in global state
    let mut project_state = state.lock().await;
    *project_state = Some(project);

    Ok(db_project)
}

#[tauri::command]
pub async fn get_project_stats(project_id: String) -> Result<ProjectStats, String> {
    use crate::schema::{assets, decisions, variant_groups};

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    // Get asset count
    let asset_count: i64 = assets::table
        .filter(assets::project_id.eq(&project_id))
        .count()
        .get_result(&mut conn)
        .map_err(|e| format!("Failed to count assets: {}", e))?;

    // Get decision counts
    let keep_count: i64 = decisions::table
        .filter(decisions::state.eq("keep"))
        .inner_join(assets::table.on(assets::id.eq(decisions::asset_id)))
        .filter(assets::project_id.eq(&project_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let remove_count: i64 = decisions::table
        .filter(decisions::state.eq("remove"))
        .inner_join(assets::table.on(assets::id.eq(decisions::asset_id)))
        .filter(assets::project_id.eq(&project_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    // Get group counts
    let duplicate_groups: i64 = variant_groups::table
        .filter(variant_groups::project_id.eq(&project_id))
        .filter(variant_groups::group_type.eq("exact"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let similar_groups: i64 = variant_groups::table
        .filter(variant_groups::project_id.eq(&project_id))
        .filter(variant_groups::group_type.eq("similar"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(ProjectStats {
        total_assets: asset_count,
        keep_count,
        remove_count,
        undecided_count: asset_count - keep_count - remove_count,
        duplicate_groups,
        similar_groups,
    })
}

#[tauri::command]
pub async fn rename_project(project_id: String, new_name: String) -> Result<(), String> {
    use crate::schema::projects::dsl::*;

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();

    diesel::update(projects.filter(id.eq(&project_id)))
        .set((name.eq(&new_name), updated_at.eq(&now)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to rename project: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_project(project_id: String) -> Result<(), String> {
    use crate::schema::{asset_groups, assets, decisions, projects, variant_groups};

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    // Delete in order to respect foreign key constraints
    // First delete asset_groups
    diesel::delete(
        asset_groups::table.filter(
            asset_groups::asset_id.eq_any(
                assets::table
                    .filter(assets::project_id.eq(&project_id))
                    .select(assets::id),
            ),
        ),
    )
    .execute(&mut conn)
    .map_err(|e| format!("Failed to delete asset groups: {}", e))?;

    // Delete decisions
    diesel::delete(
        decisions::table.filter(
            decisions::asset_id.eq_any(
                assets::table
                    .filter(assets::project_id.eq(&project_id))
                    .select(assets::id),
            ),
        ),
    )
    .execute(&mut conn)
    .map_err(|e| format!("Failed to delete decisions: {}", e))?;

    // Delete variant groups
    diesel::delete(variant_groups::table.filter(variant_groups::project_id.eq(&project_id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete variant groups: {}", e))?;

    // Delete assets
    diesel::delete(assets::table.filter(assets::project_id.eq(&project_id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete assets: {}", e))?;

    // Finally delete the project
    diesel::delete(projects::table.filter(projects::id.eq(&project_id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn duplicate_project(project_id: String, new_name: String) -> Result<DbProject, String> {
    use crate::schema::projects::dsl::*;

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    // Get the original project
    let original_project = projects
        .filter(id.eq(&project_id))
        .first::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to find project: {}", e))?;

    // Create new project with duplicated settings
    let new_project_id = format!("prj_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    let new_project = NewProject {
        id: new_project_id.clone(),
        name: new_name,
        source_path: original_project.source_path,
        output_path: original_project.output_path,
        exclude_patterns: original_project.exclude_patterns,
        file_types: original_project.file_types,
        scan_status: String::from(ScanStatus::NotStarted),
        created_at: now.clone(),
        updated_at: now,
    };

    diesel::insert_into(projects)
        .values(&new_project)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to create duplicate project: {}", e))?;

    // Return the new project
    let duplicated_project = projects
        .filter(id.eq(&new_project_id))
        .first::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to load duplicated project: {}", e))?;

    Ok(duplicated_project)
}

#[tauri::command]
pub async fn get_project_assets(
    project_id: String,
) -> Result<Vec<crate::database::models::Asset>, String> {
    use crate::database::repositories::AssetRepository;

    let asset_repo = AssetRepository::new();
    asset_repo
        .find_by_project_id(&project_id)
        .map_err(|e| format!("Failed to load assets: {}", e))
}

#[tauri::command]
pub async fn get_project_assets_paginated(
    project_id: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<crate::database::models::Asset>, String> {
    use crate::database::repositories::AssetRepository;

    let asset_repo = AssetRepository::new();
    asset_repo
        .find_by_project_id_paginated(&project_id, limit, offset)
        .map_err(|e| format!("Failed to load assets: {}", e))
}

#[tauri::command]
pub async fn get_asset_count(project_id: String) -> Result<i64, String> {
    use crate::database::repositories::AssetRepository;

    let asset_repo = AssetRepository::new();
    asset_repo
        .count_by_project_id(&project_id)
        .map_err(|e| format!("Failed to count assets: {}", e))
}

#[tauri::command]
pub async fn get_thumbnail_path(project_id: String, asset_id: String) -> Result<String, String> {
    let scanner = ScannerService::new();
    match scanner.get_thumbnail_path(&project_id, &asset_id) {
        Ok(path) => {
            if path.exists() {
                Ok(path.to_string_lossy().to_string())
            } else {
                Err(format!("Thumbnail not found for asset {}", asset_id))
            }
        }
        Err(e) => Err(format!("Failed to get thumbnail path: {}", e)),
    }
}

#[tauri::command]
pub async fn get_thumbnail_data(project_id: String, asset_id: String) -> Result<Vec<u8>, String> {
    let scanner = ScannerService::new();
    match scanner.get_thumbnail_path(&project_id, &asset_id) {
        Ok(path) => {
            if path.exists() {
                std::fs::read(&path).map_err(|e| format!("Failed to read thumbnail: {}", e))
            } else {
                Err(format!("Thumbnail not found for asset {}", asset_id))
            }
        }
        Err(e) => Err(format!("Failed to get thumbnail path: {}", e)),
    }
}

#[tauri::command]
pub async fn get_project_cache_info(project_id: String) -> Result<ProjectCacheInfo, String> {
    use crate::schema::projects::dsl::*;

    let mut conn = get_connection().map_err(|e| e.to_string())?;

    let project = projects
        .filter(id.eq(&project_id))
        .first::<DbProject>(&mut conn)
        .map_err(|e| format!("Failed to load project: {}", e))?;

    // Determine cache directory location
    let base_path = if !project.output_path.is_empty() {
        project.output_path.clone()
    } else {
        project.source_path.clone()
    };

    let cache_dir = std::path::Path::new(&base_path).join(".cullrs");
    let thumbnails_dir = cache_dir.join("thumbnails");

    // Count thumbnails if directory exists
    let thumbnail_count = if thumbnails_dir.exists() {
        std::fs::read_dir(&thumbnails_dir)
            .map_err(|e| format!("Failed to read thumbnails directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_lowercase() == "jpg")
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    };

    Ok(ProjectCacheInfo {
        cache_directory: cache_dir.to_string_lossy().to_string(),
        thumbnails_directory: thumbnails_dir.to_string_lossy().to_string(),
        thumbnail_count,
        cache_exists: cache_dir.exists(),
    })
}

#[derive(Debug, Serialize)]
pub struct ProjectStats {
    pub total_assets: i64,
    pub keep_count: i64,
    pub remove_count: i64,
    pub undecided_count: i64,
    pub duplicate_groups: i64,
    pub similar_groups: i64,
}

#[derive(Debug, Serialize)]
pub struct ProjectCacheInfo {
    pub cache_directory: String,
    pub thumbnails_directory: String,
    pub thumbnail_count: usize,
    pub cache_exists: bool,
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_project_operations() {
        // Initialize database for testing
        crate::database::connection::init_database().unwrap();

        // Create temporary directories for testing
        let temp_source = tempdir().unwrap();
        let temp_output = tempdir().unwrap();

        let source_path = temp_source.path().to_string_lossy().to_string();
        let output_path = temp_output.path().to_string_lossy().to_string();
        let project_name = "Test Project".to_string();

        // Test direct database operations
        use crate::database::models::NewProject;
        use crate::schema::projects;
        use diesel::prelude::*;

        let project_id = format!("prj_{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();

        let new_project = NewProject {
            id: project_id.clone(),
            name: project_name.clone(),
            source_path: source_path.clone(),
            output_path: output_path.clone(),
            exclude_patterns: "[]".to_string(),
            file_types: r#"["jpg","jpeg","png"]"#.to_string(),
            scan_status: String::from(ScanStatus::NotStarted),
            created_at: now.clone(),
            updated_at: now,
        };

        // Insert project into database
        let mut conn = get_connection().unwrap();
        diesel::insert_into(projects::table)
            .values(&new_project)
            .execute(&mut conn)
            .unwrap();

        // Test loading recent projects
        let recent_projects = get_recent_projects().await.unwrap();
        assert!(!recent_projects.is_empty());

        let found_project = recent_projects
            .iter()
            .find(|p| p.id == project_id)
            .expect("Project should be found in recent projects");

        assert_eq!(found_project.name, project_name);
        assert_eq!(found_project.source_path, source_path);
        assert_eq!(found_project.output_path, output_path);
    }

    #[tokio::test]
    async fn test_get_default_output_location() {
        let result = get_default_output_location().await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.ends_with("Cullrs"));
        assert!(!path.contains("/Users/john")); // Should not contain hardcoded user path

        // The path should be an absolute path (contains path separators)
        assert!(path.contains(std::path::MAIN_SEPARATOR));
    }
}
