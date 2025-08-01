use crate::database::models::{Asset, ExifData};
use chrono::Utc;
use glob::Pattern;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Unsupported file type: {extension}")]
    UnsupportedFileType { extension: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Operation cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub files_processed: usize,
    pub total_files: usize,
    pub current_file: String,
    pub estimated_time_remaining: Option<u64>, // seconds
    pub phase: ScanPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanPhase {
    Discovery,
    Processing,
    Complete,
}

pub struct ScannerService {
    progress_sender: Option<mpsc::UnboundedSender<ScanProgress>>,
    cancellation_token: Arc<AtomicBool>,
    supported_formats: HashSet<String>,
}

impl ScannerService {
    pub fn new() -> Self {
        let mut supported_formats = HashSet::new();
        supported_formats.insert("jpg".to_string());
        supported_formats.insert("jpeg".to_string());
        supported_formats.insert("png".to_string());
        supported_formats.insert("heic".to_string());
        supported_formats.insert("tiff".to_string());
        supported_formats.insert("tif".to_string());
        supported_formats.insert("webp".to_string());
        supported_formats.insert("cr3".to_string());
        supported_formats.insert("nef".to_string());
        supported_formats.insert("arw".to_string());
        supported_formats.insert("dng".to_string());

        Self {
            progress_sender: None,
            cancellation_token: Arc::new(AtomicBool::new(false)),
            supported_formats,
        }
    }

    pub fn with_progress_sender(mut self, sender: mpsc::UnboundedSender<ScanProgress>) -> Self {
        self.progress_sender = Some(sender);
        self
    }

    pub fn get_cancellation_token(&self) -> Arc<AtomicBool> {
        self.cancellation_token.clone()
    }

    pub fn cancel_scan(&self) {
        self.cancellation_token.store(true, Ordering::Relaxed);
    }

    pub async fn scan_paths(
        &self,
        project_id: &str,
        paths: &[PathBuf],
        file_types: &[String],
        exclude_patterns: &[String],
    ) -> Result<Vec<Asset>, ScanError> {
        // Check for cancellation at the start
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Validate paths
        for path in paths {
            if !path.exists() {
                return Err(ScanError::InvalidPath {
                    path: path.to_string_lossy().to_string(),
                });
            }

            if !path.is_dir() {
                return Err(ScanError::InvalidPath {
                    path: format!("{} is not a directory", path.to_string_lossy()),
                });
            }
        }

        // Compile exclude patterns
        let exclude_patterns: Result<Vec<Pattern>, _> = exclude_patterns
            .iter()
            .map(|pattern| Pattern::new(pattern))
            .collect();

        let exclude_patterns = exclude_patterns.map_err(|e| {
            ScanError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                e.to_string(),
            ))
        })?;

        // Filter file types to only supported ones
        let file_types: HashSet<String> = file_types
            .iter()
            .map(|ext| ext.to_lowercase())
            .filter(|ext| self.supported_formats.contains(ext))
            .collect();

        // Phase 1: Discovery - find all matching files
        self.send_progress(ScanProgress {
            files_processed: 0,
            total_files: 0,
            current_file: "Discovering files...".to_string(),
            estimated_time_remaining: None,
            phase: ScanPhase::Discovery,
        });

        let discovered_files = self.discover_files(paths, &file_types, &exclude_patterns)?;

        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        let total_files = discovered_files.len();

        // Phase 2: Processing - extract metadata from files
        self.send_progress(ScanProgress {
            files_processed: 0,
            total_files,
            current_file: "Processing files...".to_string(),
            estimated_time_remaining: None,
            phase: ScanPhase::Processing,
        });

        let assets = self.process_files(project_id, discovered_files)?;

        // Phase 3: Complete
        self.send_progress(ScanProgress {
            files_processed: total_files,
            total_files,
            current_file: "Scan complete".to_string(),
            estimated_time_remaining: Some(0),
            phase: ScanPhase::Complete,
        });

        Ok(assets)
    }

    fn discover_files(
        &self,
        paths: &[PathBuf],
        file_types: &HashSet<String>,
        exclude_patterns: &[Pattern],
    ) -> Result<Vec<PathBuf>, ScanError> {
        let mut discovered_files = Vec::new();

        for root_path in paths {
            if self.cancellation_token.load(Ordering::Relaxed) {
                return Err(ScanError::Cancelled);
            }

            for entry in WalkDir::new(root_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                let path = entry.path();

                // Skip directories
                if !path.is_file() {
                    continue;
                }

                // Check if path matches any exclude pattern
                let path_str = path.to_string_lossy();
                if exclude_patterns
                    .iter()
                    .any(|pattern| pattern.matches(&path_str))
                {
                    continue;
                }

                // Check file extension
                if let Some(extension) = path.extension() {
                    let ext = extension.to_string_lossy().to_lowercase();
                    if file_types.contains(&ext) {
                        discovered_files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(discovered_files)
    }

    fn process_files(
        &self,
        project_id: &str,
        files: Vec<PathBuf>,
    ) -> Result<Vec<Asset>, ScanError> {
        let total_files = files.len();
        let processed_count = Arc::new(AtomicUsize::new(0));
        let _assets = Arc::new(Mutex::new(Vec::<Asset>::new()));
        let start_time = std::time::Instant::now();

        // Process files in parallel using rayon
        let results: Result<Vec<_>, ScanError> = files
            .into_par_iter()
            .map(|file_path| {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                let asset = self.process_single_file(project_id, &file_path)?;

                // Update progress
                let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                let elapsed = start_time.elapsed().as_secs();
                let estimated_remaining = if current_count > 0 && elapsed > 0 {
                    let rate = current_count as f64 / elapsed as f64;
                    let remaining_files = total_files - current_count;
                    Some((remaining_files as f64 / rate) as u64)
                } else {
                    None
                };

                self.send_progress(ScanProgress {
                    files_processed: current_count,
                    total_files,
                    current_file: file_path.to_string_lossy().to_string(),
                    estimated_time_remaining: estimated_remaining,
                    phase: ScanPhase::Processing,
                });

                Ok(asset)
            })
            .collect();

        match results {
            Ok(assets) => Ok(assets),
            Err(e) => Err(e),
        }
    }

    fn process_single_file(&self, project_id: &str, file_path: &Path) -> Result<Asset, ScanError> {
        // Get file metadata
        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len() as i32;

        // Try to get image dimensions
        let (width, height) = self.get_image_dimensions(file_path)?;

        // Extract EXIF data (basic implementation for now)
        let exif_data = self.extract_basic_exif(file_path);

        let asset_id = format!("ast_{}", Uuid::new_v4().simple());
        let now = Utc::now().to_rfc3339();

        Ok(Asset {
            id: asset_id,
            project_id: project_id.to_string(),
            path: file_path.to_string_lossy().to_string(),
            hash: None,            // Will be computed later
            perceptual_hash: None, // Will be computed later
            size: file_size,
            width: width as i32,
            height: height as i32,
            exif_data: exif_data.map(|data| serde_json::to_string(&data).unwrap_or_default()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    fn get_image_dimensions(&self, file_path: &Path) -> Result<(u32, u32), ScanError> {
        // Try to get dimensions without fully loading the image
        match image::image_dimensions(file_path) {
            Ok(dimensions) => Ok(dimensions),
            Err(e) => {
                // For unsupported formats, try to open the image
                match image::open(file_path) {
                    Ok(img) => Ok((img.width(), img.height())),
                    Err(_) => {
                        // If we can't read the image, return default dimensions
                        log::warn!(
                            "Could not read dimensions for {}: {}",
                            file_path.display(),
                            e
                        );
                        Ok((0, 0))
                    }
                }
            }
        }
    }

    fn extract_basic_exif(&self, _file_path: &Path) -> Option<ExifData> {
        // Basic EXIF extraction - for now just return None
        // This will be implemented in a later task
        None
    }

    fn send_progress(&self, progress: ScanProgress) {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(progress);
        }
    }

    pub fn is_supported_format(&self, file_path: &Path) -> bool {
        if let Some(extension) = file_path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            self.supported_formats.contains(&ext)
        } else {
            false
        }
    }

    pub fn get_supported_formats(&self) -> &HashSet<String> {
        &self.supported_formats
    }
}

impl Default for ScannerService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = ScannerService::new();

        let assets = scanner
            .scan_paths(
                "test_project",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string(), "png".to_string()],
                &[],
            )
            .await
            .unwrap();

        assert_eq!(assets.len(), 0);
    }

    #[tokio::test]
    async fn test_scan_with_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let included_file = temp_dir.path().join("included.jpg");
        let excluded_file = temp_dir.path().join("excluded.tmp.jpg");

        fs::write(&included_file, b"fake jpg content").unwrap();
        fs::write(&excluded_file, b"fake jpg content").unwrap();

        let scanner = ScannerService::new();

        let assets = scanner
            .scan_paths(
                "test_project",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string()],
                &["*.tmp.*".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(assets.len(), 1);
        assert!(assets[0].path.contains("included.jpg"));
    }

    #[tokio::test]
    async fn test_file_type_filtering() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files with different extensions
        let jpg_file = temp_dir.path().join("test.jpg");
        let png_file = temp_dir.path().join("test.png");
        let txt_file = temp_dir.path().join("test.txt");

        fs::write(&jpg_file, b"fake jpg content").unwrap();
        fs::write(&png_file, b"fake png content").unwrap();
        fs::write(&txt_file, b"text content").unwrap();

        let scanner = ScannerService::new();

        let assets = scanner
            .scan_paths(
                "test_project",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string(), "png".to_string()],
                &[],
            )
            .await
            .unwrap();

        assert_eq!(assets.len(), 2);

        let paths: Vec<&str> = assets.iter().map(|a| a.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("test.jpg")));
        assert!(paths.iter().any(|p| p.contains("test.png")));
        assert!(!paths.iter().any(|p| p.contains("test.txt")));
    }

    #[test]
    fn test_supported_format_detection() {
        let scanner = ScannerService::new();

        assert!(scanner.is_supported_format(Path::new("test.jpg")));
        assert!(scanner.is_supported_format(Path::new("test.JPEG")));
        assert!(scanner.is_supported_format(Path::new("test.png")));
        assert!(scanner.is_supported_format(Path::new("test.heic")));
        assert!(!scanner.is_supported_format(Path::new("test.txt")));
        assert!(!scanner.is_supported_format(Path::new("test")));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let temp_dir = TempDir::new().unwrap();

        // Create many test files to ensure cancellation can be tested
        for i in 0..100 {
            let file_path = temp_dir.path().join(format!("test_{}.jpg", i));
            fs::write(&file_path, b"fake jpg content").unwrap();
        }

        let scanner = ScannerService::new();
        let temp_path = temp_dir.path().to_path_buf();

        // Cancel immediately before starting scan
        scanner.cancel_scan();

        let result = scanner
            .scan_paths("test_project", &[temp_path], &["jpg".to_string()], &[])
            .await;

        assert!(matches!(result, Err(ScanError::Cancelled)));
    }
}
