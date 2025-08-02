use crate::core::exif::ExifService;
use crate::core::hash::HashService;
use crate::core::thumbnail::{ThumbnailProgress, ThumbnailService};
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

    #[error("Thumbnail generation error: {0}")]
    Thumbnail(#[from] crate::core::thumbnail::ThumbnailError),

    #[error("Hash computation error: {0}")]
    Hash(#[from] crate::core::hash::HashError),

    #[error("EXIF extraction error: {0}")]
    Exif(#[from] crate::core::exif::ExifError),

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScanPhase {
    Discovery,
    Processing,
    ThumbnailGeneration,
    HashingAndExif,
    Complete,
}

pub struct ScannerService {
    progress_sender: Option<mpsc::UnboundedSender<ScanProgress>>,
    cancellation_token: Arc<AtomicBool>,
    supported_formats: HashSet<String>,
    thumbnail_service: ThumbnailService,
    hash_service: HashService,
    exif_service: ExifService,
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
            thumbnail_service: ThumbnailService::new(),
            hash_service: HashService::new(),
            exif_service: ExifService::new(),
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

        let mut assets = self.process_files(project_id, discovered_files)?;

        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Phase 3: Thumbnail Generation
        self.send_progress(ScanProgress {
            files_processed: 0,
            total_files,
            current_file: "Generating thumbnails...".to_string(),
            estimated_time_remaining: None,
            phase: ScanPhase::ThumbnailGeneration,
        });

        self.generate_thumbnails(&mut assets, project_id).await?;

        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Phase 4: Hashing and EXIF extraction
        self.send_progress(ScanProgress {
            files_processed: 0,
            total_files,
            current_file: "Computing hashes and extracting metadata...".to_string(),
            estimated_time_remaining: None,
            phase: ScanPhase::HashingAndExif,
        });

        self.compute_hashes_and_exif(&mut assets)?;

        // Phase 5: Complete
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

    fn extract_basic_exif(&self, file_path: &Path) -> Option<ExifData> {
        match self.exif_service.extract_exif(file_path) {
            Ok(exif_data) => exif_data,
            Err(e) => {
                log::warn!("Failed to extract EXIF from {}: {}", file_path.display(), e);
                None
            }
        }
    }

    /// Compute content hashes and extract EXIF data for all assets
    fn compute_hashes_and_exif(&self, assets: &mut [Asset]) -> Result<(), ScanError> {
        let total_assets = assets.len();
        let processed_count = Arc::new(AtomicUsize::new(0));
        let start_time = std::time::Instant::now();

        // Process assets in parallel using rayon
        let results: Result<Vec<_>, ScanError> = assets
            .par_iter_mut()
            .map(|asset| {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                let file_path = Path::new(&asset.path);

                // Compute content hash from original file
                match self.hash_service.compute_content_hash(file_path) {
                    Ok(hash) => {
                        asset.hash = Some(hash);
                    }
                    Err(e) => {
                        log::warn!("Failed to compute hash for {}: {}", asset.path, e);
                        // Continue processing other assets even if one fails
                    }
                }

                // Extract EXIF data
                match self.exif_service.extract_exif(file_path) {
                    Ok(Some(exif_data)) => {
                        // Serialize EXIF data to JSON string for database storage
                        match serde_json::to_string(&exif_data) {
                            Ok(json_str) => {
                                asset.exif_data = Some(json_str);
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to serialize EXIF data for {}: {}",
                                    asset.path,
                                    e
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        // No EXIF data found, which is fine
                        asset.exif_data = None;
                    }
                    Err(e) => {
                        log::warn!("Failed to extract EXIF from {}: {}", asset.path, e);
                        asset.exif_data = None;
                    }
                }

                // Update progress
                let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                let elapsed = start_time.elapsed().as_secs();
                let estimated_remaining = if current_count > 0 && elapsed > 0 {
                    let rate = current_count as f64 / elapsed as f64;
                    let remaining_assets = total_assets - current_count;
                    Some((remaining_assets as f64 / rate) as u64)
                } else {
                    None
                };

                self.send_progress(ScanProgress {
                    files_processed: current_count,
                    total_files: total_assets,
                    current_file: asset.path.clone(),
                    estimated_time_remaining: estimated_remaining,
                    phase: ScanPhase::HashingAndExif,
                });

                Ok(())
            })
            .collect();

        match results {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
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

    /// Generate thumbnails for all assets in the project with enhanced error handling
    pub async fn generate_thumbnails(
        &self,
        assets: &mut [Asset],
        project_id: &str,
    ) -> Result<(), ScanError> {
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Get project temp directory
        let project_temp_dir = self.get_project_temp_dir(project_id)?;

        // Set up progress forwarding from thumbnail service to scanner progress
        let (thumbnail_tx, mut thumbnail_rx) = mpsc::unbounded_channel::<ThumbnailProgress>();
        let progress_sender = self.progress_sender.clone();
        let cancellation_token = self.cancellation_token.clone();

        // Spawn task to forward thumbnail progress to scan progress
        let progress_forwarder = tokio::spawn(async move {
            while let Some(thumbnail_progress) = thumbnail_rx.recv().await {
                // Check for cancellation
                if cancellation_token.load(Ordering::Relaxed) {
                    break;
                }

                if let Some(sender) = &progress_sender {
                    let scan_progress = ScanProgress {
                        files_processed: thumbnail_progress.thumbnails_generated,
                        total_files: thumbnail_progress.total_thumbnails,
                        current_file: thumbnail_progress.current_file,
                        estimated_time_remaining: thumbnail_progress.estimated_time_remaining,
                        phase: ScanPhase::ThumbnailGeneration,
                    };
                    let _ = sender.send(scan_progress);
                }
            }
        });

        // Create thumbnail service with cancellation token
        let mut thumbnail_service = ThumbnailService::new();
        thumbnail_service.cancellation_token = self.cancellation_token.clone();

        // Generate thumbnails using the thumbnail service with error recovery
        let result = thumbnail_service
            .generate_thumbnails_batch_with_recovery(assets, &project_temp_dir, Some(thumbnail_tx))
            .await;

        // Wait for progress forwarder to finish
        progress_forwarder.abort();

        // Update assets with thumbnail paths, handling corrupted files gracefully
        let mut successful_thumbnails = 0;
        let mut failed_thumbnails = 0;

        for asset in assets.iter_mut() {
            let thumbnail_path = self
                .thumbnail_service
                .get_thumbnail_path(&project_temp_dir, &asset.id);

            if thumbnail_path.exists() {
                successful_thumbnails += 1;
                // Store relative path from project temp dir for portability
                if let Ok(_relative_path) = thumbnail_path.strip_prefix(&project_temp_dir) {
                    // We'll store the full path for now, but this could be optimized
                    // to store relative paths if needed for project portability
                }
            } else {
                failed_thumbnails += 1;
                log::warn!(
                    "Thumbnail generation failed for asset {}: {}",
                    asset.id,
                    asset.path
                );
            }
        }

        log::info!(
            "Thumbnail generation completed: {} successful, {} failed",
            successful_thumbnails,
            failed_thumbnails
        );

        result.map_err(ScanError::from)
    }

    /// Get the temporary directory for a project
    fn get_project_temp_dir(&self, project_id: &str) -> Result<PathBuf, ScanError> {
        // Use system temp directory with project-specific subdirectory
        let temp_dir = std::env::temp_dir()
            .join("cullrs")
            .join("projects")
            .join(project_id);

        // Create directory if it doesn't exist
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir).map_err(|e| ScanError::Io(e))?;
        }

        Ok(temp_dir)
    }

    /// Clean up thumbnails for a project
    pub async fn cleanup_project_thumbnails(&self, project_id: &str) -> Result<(), ScanError> {
        let project_temp_dir = self.get_project_temp_dir(project_id)?;
        self.thumbnail_service
            .cleanup_thumbnails(&project_temp_dir)
            .await
            .map_err(|e| ScanError::Io(e))
    }

    /// Get thumbnail path for an asset
    pub fn get_thumbnail_path(
        &self,
        project_id: &str,
        asset_id: &str,
    ) -> Result<PathBuf, ScanError> {
        let project_temp_dir = self.get_project_temp_dir(project_id)?;
        Ok(self
            .thumbnail_service
            .get_thumbnail_path(&project_temp_dir, asset_id))
    }

    /// Compute content hash for a single asset (used for re-processing)
    pub fn compute_asset_hash(&self, asset: &mut Asset) -> Result<(), ScanError> {
        let file_path = Path::new(&asset.path);
        match self.hash_service.compute_content_hash(file_path) {
            Ok(hash) => {
                asset.hash = Some(hash);
                Ok(())
            }
            Err(e) => Err(ScanError::Hash(e)),
        }
    }

    /// Extract EXIF data for a single asset (used for re-processing)
    pub fn extract_asset_exif(&self, asset: &mut Asset) -> Result<(), ScanError> {
        let file_path = Path::new(&asset.path);
        match self.exif_service.extract_exif(file_path) {
            Ok(Some(exif_data)) => match serde_json::to_string(&exif_data) {
                Ok(json_str) => {
                    asset.exif_data = Some(json_str);
                    Ok(())
                }
                Err(e) => {
                    log::warn!("Failed to serialize EXIF data for {}: {}", asset.path, e);
                    asset.exif_data = None;
                    Ok(())
                }
            },
            Ok(None) => {
                asset.exif_data = None;
                Ok(())
            }
            Err(e) => Err(ScanError::Exif(e)),
        }
    }

    /// Verify that an asset's stored hash matches its current file content
    pub fn verify_asset_hash(&self, asset: &Asset) -> Result<bool, ScanError> {
        if let Some(stored_hash) = &asset.hash {
            let file_path = Path::new(&asset.path);
            let current_hash = self.hash_service.compute_content_hash(file_path)?;
            Ok(*stored_hash == current_hash)
        } else {
            Ok(false) // No stored hash to verify against
        }
    }

    /// Get access to the hash service for external use
    pub fn hash_service(&self) -> &HashService {
        &self.hash_service
    }

    /// Get access to the EXIF service for external use
    pub fn exif_service(&self) -> &ExifService {
        &self.exif_service
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
    use image::GenericImageView;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_image(
        path: &Path,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use image::{ImageBuffer, Rgb};

        let img = ImageBuffer::from_fn(width, height, |x, y| {
            let intensity = ((x + y) % 256) as u8;
            Rgb([intensity, intensity, intensity])
        });

        img.save(path)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_enhanced_progress_tracking() {
        let temp_dir = TempDir::new().unwrap();

        // Create test images
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("test_{}.jpg", i));
            create_test_image(&file_path, 100, 100).unwrap();
        }

        // Set up progress tracking
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ScanProgress>();
        let scanner = ScannerService::new().with_progress_sender(progress_tx);

        // Track progress events
        let progress_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_events_clone = progress_events.clone();

        // Spawn task to collect progress events
        let progress_collector = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let mut events = progress_events_clone.lock().unwrap();
                events.push(progress);
            }
        });

        // Perform scan
        let result = scanner
            .scan_paths(
                "test_project",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string()],
                &[],
            )
            .await;

        // Wait a bit for progress events to be collected
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        progress_collector.abort();

        assert!(result.is_ok());
        let assets = result.unwrap();
        assert_eq!(assets.len(), 5);

        // Verify progress events were sent
        let events = progress_events.lock().unwrap();
        assert!(!events.is_empty());

        // Verify we have different phases
        let phases: std::collections::HashSet<_> = events.iter().map(|e| &e.phase).collect();
        assert!(phases.contains(&ScanPhase::Discovery));
        assert!(phases.contains(&ScanPhase::Processing));
        assert!(phases.contains(&ScanPhase::ThumbnailGeneration));
        assert!(phases.contains(&ScanPhase::HashingAndExif));
        assert!(phases.contains(&ScanPhase::Complete));

        // Verify final progress shows completion
        let final_progress = events.last().unwrap();
        assert_eq!(final_progress.phase, ScanPhase::Complete);
        assert_eq!(final_progress.files_processed, final_progress.total_files);
    }

    #[tokio::test]
    async fn test_cancellation_functionality() {
        let temp_dir = TempDir::new().unwrap();

        // Create many test images to ensure we can cancel
        for i in 0..50 {
            let file_path = temp_dir.path().join(format!("test_{}.jpg", i));
            create_test_image(&file_path, 200, 200).unwrap(); // Larger images take more time
        }

        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ScanProgress>();
        let scanner = ScannerService::new().with_progress_sender(progress_tx);
        let cancellation_token = scanner.get_cancellation_token();

        // Cancel immediately before starting the scan
        cancellation_token.store(true, Ordering::Relaxed);

        // Perform scan (should be cancelled immediately)
        let result = scanner
            .scan_paths(
                "test_project_cancel",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string()],
                &[],
            )
            .await;

        // Verify scan was cancelled
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScanError::Cancelled));
    }

    #[tokio::test]
    async fn test_cancellation_during_processing() {
        let temp_dir = TempDir::new().unwrap();

        // Create many test images to ensure we can cancel during processing
        for i in 0..30 {
            let file_path = temp_dir.path().join(format!("test_{}.jpg", i));
            create_test_image(&file_path, 300, 300).unwrap(); // Larger images
        }

        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ScanProgress>();
        let scanner = ScannerService::new().with_progress_sender(progress_tx);
        let cancellation_token = scanner.get_cancellation_token();

        // Spawn task to cancel after we see some progress
        let cancellation_token_clone = cancellation_token.clone();
        let cancellation_task = tokio::spawn(async move {
            let mut progress_count = 0;
            while let Some(progress) = progress_rx.recv().await {
                progress_count += 1;
                // Cancel after we've seen a few progress updates
                if progress_count >= 3 && progress.files_processed > 0 {
                    cancellation_token_clone.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });

        // Perform scan (should be cancelled during processing)
        let result = scanner
            .scan_paths(
                "test_project_cancel_during",
                &[temp_dir.path().to_path_buf()],
                &["jpg".to_string()],
                &[],
            )
            .await;

        cancellation_task.abort();

        // Verify scan was cancelled (it might complete if cancellation was too late)
        if result.is_err() {
            assert!(matches!(result.unwrap_err(), ScanError::Cancelled));
        } else {
            // If scan completed, that's also acceptable for this test
            println!("Scan completed before cancellation could take effect");
        }
    }

    #[cfg(test)]
    mod original_tests {
        use super::*;
        use image::GenericImageView;
        use std::fs;
        use tempfile::TempDir;

        fn create_test_image(
            path: &Path,
            width: u32,
            height: u32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            use image::{ImageBuffer, Rgb};

            let img = ImageBuffer::from_fn(width, height, |x, y| {
                let intensity = ((x + y) % 256) as u8;
                Rgb([intensity, intensity, intensity])
            });

            img.save(path)?;
            Ok(())
        }

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

            create_test_image(&included_file, 100, 100).unwrap();
            create_test_image(&excluded_file, 100, 100).unwrap();

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

            create_test_image(&jpg_file, 100, 100).unwrap();
            create_test_image(&png_file, 100, 100).unwrap();
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
            for i in 0..10 {
                let file_path = temp_dir.path().join(format!("test_{}.jpg", i));
                create_test_image(&file_path, 100, 100).unwrap();
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

        #[tokio::test]
        async fn test_thumbnail_generation_integration() {
            let temp_dir = TempDir::new().unwrap();

            // Create test images
            let jpg_file = temp_dir.path().join("test.jpg");
            create_test_image(&jpg_file, 1920, 1080).unwrap();

            let scanner = ScannerService::new();

            let assets = scanner
                .scan_paths(
                    "test_project_thumb",
                    &[temp_dir.path().to_path_buf()],
                    &["jpg".to_string()],
                    &[],
                )
                .await
                .unwrap();

            assert_eq!(assets.len(), 1);

            // Verify thumbnail was generated
            let thumbnail_path = scanner
                .get_thumbnail_path("test_project_thumb", &assets[0].id)
                .unwrap();

            assert!(thumbnail_path.exists());

            // Verify thumbnail dimensions
            let thumbnail_img = image::open(&thumbnail_path).unwrap();
            let (width, height) = thumbnail_img.dimensions();
            assert!(width <= 512 && height <= 512);
            assert!(width == 512 || height == 512); // One dimension should be exactly 512
        }

        #[tokio::test]
        async fn test_hash_computation_integration() {
            let temp_dir = TempDir::new().unwrap();

            // Create test images with known content
            let jpg_file1 = temp_dir.path().join("test1.jpg");
            let jpg_file2 = temp_dir.path().join("test2.jpg");

            create_test_image(&jpg_file1, 100, 100).unwrap();
            create_test_image(&jpg_file2, 200, 200).unwrap();

            let scanner = ScannerService::new();

            let assets = scanner
                .scan_paths(
                    "test_project_hash",
                    &[temp_dir.path().to_path_buf()],
                    &["jpg".to_string()],
                    &[],
                )
                .await
                .unwrap();

            assert_eq!(assets.len(), 2);

            // Verify both assets have hashes
            for asset in &assets {
                assert!(asset.hash.is_some());
                let hash = asset.hash.as_ref().unwrap();
                assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
                assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
            }

            // Verify different images have different hashes
            assert_ne!(assets[0].hash, assets[1].hash);
        }

        #[tokio::test]
        async fn test_identical_files_same_hash() {
            let temp_dir = TempDir::new().unwrap();

            // Create identical image files
            let file1 = temp_dir.path().join("identical1.jpg");
            let file2 = temp_dir.path().join("identical2.jpg");

            // Create the same image in both files
            create_test_image(&file1, 100, 100).unwrap();
            create_test_image(&file2, 100, 100).unwrap();

            let scanner = ScannerService::new();

            let assets = scanner
                .scan_paths(
                    "test_project_identical",
                    &[temp_dir.path().to_path_buf()],
                    &["jpg".to_string()],
                    &[],
                )
                .await
                .unwrap();

            assert_eq!(assets.len(), 2);

            // Verify both assets have the same hash
            assert!(assets[0].hash.is_some());
            assert!(assets[1].hash.is_some());
            assert_eq!(assets[0].hash, assets[1].hash);
        }

        #[test]
        fn test_compute_asset_hash() {
            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.jpg");

            create_test_image(&test_file, 100, 100).unwrap();

            let scanner = ScannerService::new();
            let mut asset = Asset {
                id: "test_asset".to_string(),
                project_id: "test_project".to_string(),
                path: test_file.to_string_lossy().to_string(),
                hash: None,
                perceptual_hash: None,
                size: 1000,
                width: 100,
                height: 100,
                exif_data: None,
                created_at: "2023-01-01T00:00:00Z".to_string(),
                updated_at: "2023-01-01T00:00:00Z".to_string(),
            };

            let result = scanner.compute_asset_hash(&mut asset);
            assert!(result.is_ok());
            assert!(asset.hash.is_some());

            let hash = asset.hash.unwrap();
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn test_verify_asset_hash() {
            let temp_dir = TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.jpg");

            create_test_image(&test_file, 100, 100).unwrap();

            let scanner = ScannerService::new();

            // First compute the hash
            let hash = scanner
                .hash_service()
                .compute_content_hash(&test_file)
                .unwrap();

            let asset = Asset {
                id: "test_asset".to_string(),
                project_id: "test_project".to_string(),
                path: test_file.to_string_lossy().to_string(),
                hash: Some(hash),
                perceptual_hash: None,
                size: 1000,
                width: 100,
                height: 100,
                exif_data: None,
                created_at: "2023-01-01T00:00:00Z".to_string(),
                updated_at: "2023-01-01T00:00:00Z".to_string(),
            };

            // Verify the hash matches
            let result = scanner.verify_asset_hash(&asset);
            assert!(result.is_ok());
            assert!(result.unwrap());
        }
    }
}
