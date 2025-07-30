use crate::core::{
    image::{ImageHash, ImageMetadata},
    project::{Project, ProjectConfig, ScanProgress},
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// Global state for the current project
pub type ProjectState = Arc<Mutex<Option<Project>>>;

#[tauri::command]
pub async fn create_project(
    source_dir: String,
    output_dir: String,
    project_name: String,
    state: State<'_, ProjectState>,
) -> Result<ProjectConfig, String> {
    let project = Project::new(source_dir, output_dir, project_name).map_err(|e| e.to_string())?;

    let config = project.config.clone();

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
