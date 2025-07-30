use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
    pub created_at: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub current_file: String,
    pub processed: usize,
    pub total: usize,
    pub percentage: f64,
    pub is_complete: bool,
}

#[derive(Debug)]
pub struct Project {
    pub config: ProjectConfig,
    pub images: Arc<RwLock<HashMap<PathBuf, crate::core::image::ImageMetadata>>>,
    pub scan_progress: Arc<RwLock<ScanProgress>>,
}

impl Project {
    pub fn new(
        source_dir: String,
        output_dir: String,
        name: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let source_path = PathBuf::from(&source_dir);
        let output_path = PathBuf::from(&output_dir);

        // Validate directories exist
        if !source_path.exists() {
            return Err(format!("Source directory does not exist: {}", source_dir).into());
        }

        // Create output directory if it doesn't exist
        if !output_path.exists() {
            std::fs::create_dir_all(&output_path)?;
        }

        let config = ProjectConfig {
            name,
            source_dir: source_path,
            output_dir: output_path,
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        };

        let initial_progress = ScanProgress {
            current_file: String::new(),
            processed: 0,
            total: 0,
            percentage: 0.0,
            is_complete: false,
        };

        Ok(Project {
            config,
            images: Arc::new(RwLock::new(HashMap::new())),
            scan_progress: Arc::new(RwLock::new(initial_progress)),
        })
    }

    pub async fn scan_images(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::core::image::ImageMetadata;

        // First pass: count total files
        let image_files = self.find_image_files(&self.config.source_dir)?;
        let total = image_files.len();

        // Update progress with total count
        {
            let mut progress = self.scan_progress.write().await;
            progress.total = total;
            progress.processed = 0;
            progress.percentage = 0.0;
            progress.is_complete = false;
        }

        // Second pass: process each image
        for (index, file_path) in image_files.iter().enumerate() {
            // Update progress
            {
                let mut progress = self.scan_progress.write().await;
                progress.current_file = file_path.to_string_lossy().to_string();
                progress.processed = index;
                progress.percentage = (index as f64 / total as f64) * 100.0;
            }

            // Process the image
            let path_str = file_path.to_string_lossy().to_string();
            let metadata_result = ImageMetadata::from_path(&path_str);
            match metadata_result {
                Ok(metadata) => {
                    let mut images = self.images.write().await;
                    images.insert(file_path.clone(), metadata);
                }
                Err(e) => {
                    // Log error but continue processing
                    eprintln!("Failed to process {}: {}", file_path.display(), e);
                }
            }
        }

        // Mark scan as complete
        {
            let mut progress = self.scan_progress.write().await;
            progress.processed = total;
            progress.percentage = 100.0;
            progress.is_complete = true;
            progress.current_file = String::new();
        }

        Ok(())
    }

    pub async fn get_scan_progress(&self) -> ScanProgress {
        self.scan_progress.read().await.clone()
    }

    fn find_image_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let mut image_files = Vec::new();
        let image_extensions = vec![
            "jpg", "jpeg", "png", "tiff", "tif", "bmp", "gif", "webp", "raw", "cr2", "nef", "arw",
        ];

        fn scan_directory(
            dir: &Path,
            image_files: &mut Vec<PathBuf>,
            extensions: &[&str],
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Recursively scan subdirectories
                    scan_directory(&path, image_files, extensions)?;
                } else if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if let Some(ext_str) = extension.to_str() {
                            if extensions.contains(&ext_str.to_lowercase().as_str()) {
                                image_files.push(path);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        scan_directory(dir, &mut image_files, &image_extensions)?;
        Ok(image_files)
    }
}
