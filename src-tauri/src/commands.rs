use crate::core::{
    image::{ImageHash, ImageMetadata},
    project::{Project, ProjectConfig, ScanProgress},
};
use crate::database::{
    connection::get_connection,
    models::{NewProject, Project as DbProject, ScanStatus},
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

// Global state for the current project
pub type ProjectState = Arc<Mutex<Option<Project>>>;

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
