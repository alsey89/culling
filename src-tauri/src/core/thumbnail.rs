use crate::database::models::Asset;
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Project directory not found: {path}")]
    ProjectDirectoryNotFound { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailProgress {
    pub thumbnails_generated: usize,
    pub total_thumbnails: usize,
    pub current_file: String,
    pub estimated_time_remaining: Option<u64>, // seconds
}

pub struct ThumbnailService {
    thumbnail_size: u32,
    quality: u8,
    progress_sender: Option<mpsc::UnboundedSender<ThumbnailProgress>>,
    pub cancellation_token: Arc<AtomicBool>,
    cache: Arc<Mutex<HashMap<String, PathBuf>>>, // asset_id -> thumbnail_path
}

impl ThumbnailService {
    pub fn new() -> Self {
        Self {
            thumbnail_size: 512,
            quality: 85,
            progress_sender: None,
            cancellation_token: Arc::new(AtomicBool::new(false)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_progress_sender(
        mut self,
        sender: mpsc::UnboundedSender<ThumbnailProgress>,
    ) -> Self {
        self.progress_sender = Some(sender);
        self
    }

    pub fn get_cancellation_token(&self) -> Arc<AtomicBool> {
        self.cancellation_token.clone()
    }

    pub fn cancel_generation(&self) {
        self.cancellation_token.store(true, Ordering::Relaxed);
    }

    /// Generate a single thumbnail from an original image file
    pub async fn generate_thumbnail(
        &self,
        original_path: &Path,
        thumbnail_path: &Path,
    ) -> Result<(), ThumbnailError> {
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ThumbnailError::Cancelled);
        }

        // Validate input path
        if !original_path.exists() {
            return Err(ThumbnailError::InvalidPath {
                path: original_path.to_string_lossy().to_string(),
            });
        }

        // Create thumbnail directory if it doesn't exist
        if let Some(parent) = thumbnail_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Load and process the image
        let img = self.load_image(original_path)?;
        let thumbnail = self.resize_image(img, self.thumbnail_size)?;

        // Save thumbnail with JPEG format for consistent size and quality
        self.save_thumbnail(&thumbnail, thumbnail_path)?;

        Ok(())
    }

    /// Generate thumbnails for multiple assets in parallel
    pub async fn generate_thumbnails_batch(
        &self,
        assets: &[Asset],
        project_temp_dir: &Path,
        progress_sender: Option<mpsc::UnboundedSender<ThumbnailProgress>>,
    ) -> Result<(), ThumbnailError> {
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ThumbnailError::Cancelled);
        }

        // Ensure project temp directory and thumbnails subdirectory exist
        if !project_temp_dir.exists() {
            fs::create_dir_all(project_temp_dir)?;
        }

        let thumbnails_dir = project_temp_dir.join("thumbnails");
        if !thumbnails_dir.exists() {
            fs::create_dir_all(&thumbnails_dir)?;
        }

        let total_assets = assets.len();
        let processed_count = Arc::new(AtomicUsize::new(0));
        let start_time = std::time::Instant::now();
        let cache = self.cache.clone();

        // Send initial progress
        if let Some(sender) = &progress_sender {
            let _ = sender.send(ThumbnailProgress {
                thumbnails_generated: 0,
                total_thumbnails: total_assets,
                current_file: "Starting thumbnail generation...".to_string(),
                estimated_time_remaining: None,
            });
        }

        // Process assets in parallel using rayon
        let results: Result<Vec<_>, ThumbnailError> = assets
            .par_iter()
            .map(|asset| {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    return Err(ThumbnailError::Cancelled);
                }

                let original_path = Path::new(&asset.path);
                let thumbnail_path = self.get_thumbnail_path(project_temp_dir, &asset.id);

                // Skip if thumbnail already exists and is newer than original
                if self.is_thumbnail_valid(&thumbnail_path, original_path)? {
                    // Update cache
                    {
                        let mut cache_guard = cache.lock().unwrap();
                        cache_guard.insert(asset.id.clone(), thumbnail_path.clone());
                    }

                    // Update progress
                    let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                    self.send_progress_update(
                        current_count,
                        total_assets,
                        &asset.path,
                        start_time,
                        &progress_sender,
                    );

                    return Ok(());
                }

                // Generate thumbnail
                let img = self.load_image(original_path)?;
                let thumbnail = self.resize_image(img, self.thumbnail_size)?;
                self.save_thumbnail(&thumbnail, &thumbnail_path)?;

                // Update cache
                {
                    let mut cache_guard = cache.lock().unwrap();
                    cache_guard.insert(asset.id.clone(), thumbnail_path);
                }

                // Update progress
                let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                self.send_progress_update(
                    current_count,
                    total_assets,
                    &asset.path,
                    start_time,
                    &progress_sender,
                );

                Ok(())
            })
            .collect();

        match results {
            Ok(_) => {
                // Send completion progress
                if let Some(sender) = &progress_sender {
                    let _ = sender.send(ThumbnailProgress {
                        thumbnails_generated: total_assets,
                        total_thumbnails: total_assets,
                        current_file: "Thumbnail generation complete".to_string(),
                        estimated_time_remaining: Some(0),
                    });
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Generate thumbnails with error recovery for corrupted files
    pub async fn generate_thumbnails_batch_with_recovery(
        &self,
        assets: &[Asset],
        project_temp_dir: &Path,
        progress_sender: Option<mpsc::UnboundedSender<ThumbnailProgress>>,
    ) -> Result<(), ThumbnailError> {
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ThumbnailError::Cancelled);
        }

        // Ensure project temp directory and thumbnails subdirectory exist
        if !project_temp_dir.exists() {
            fs::create_dir_all(project_temp_dir)?;
        }

        let thumbnails_dir = project_temp_dir.join("thumbnails");
        if !thumbnails_dir.exists() {
            fs::create_dir_all(&thumbnails_dir)?;
        }

        let total_assets = assets.len();
        let processed_count = Arc::new(AtomicUsize::new(0));
        let error_count = Arc::new(AtomicUsize::new(0));
        let start_time = std::time::Instant::now();
        let cache = self.cache.clone();

        // Send initial progress
        if let Some(sender) = &progress_sender {
            let _ = sender.send(ThumbnailProgress {
                thumbnails_generated: 0,
                total_thumbnails: total_assets,
                current_file: "Starting thumbnail generation...".to_string(),
                estimated_time_remaining: None,
            });
        }

        // Process assets in parallel using rayon with error recovery
        let _results: Vec<_> = assets
            .par_iter()
            .map(|asset| {
                if self.cancellation_token.load(Ordering::Relaxed) {
                    return Err(ThumbnailError::Cancelled);
                }

                let original_path = Path::new(&asset.path);
                let thumbnail_path = self.get_thumbnail_path(project_temp_dir, &asset.id);

                // Skip if thumbnail already exists and is newer than original
                match self.is_thumbnail_valid(&thumbnail_path, original_path) {
                    Ok(true) => {
                        // Update cache
                        {
                            let mut cache_guard = cache.lock().unwrap();
                            cache_guard.insert(asset.id.clone(), thumbnail_path.clone());
                        }

                        // Update progress
                        let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                        self.send_progress_update(
                            current_count,
                            total_assets,
                            &asset.path,
                            start_time,
                            &progress_sender,
                        );

                        return Ok(());
                    }
                    Ok(false) => {
                        // Need to generate thumbnail
                    }
                    Err(e) => {
                        log::warn!(
                            "Error checking thumbnail validity for {}: {}",
                            asset.path,
                            e
                        );
                        // Continue with generation attempt
                    }
                }

                // Attempt to generate thumbnail with error recovery
                let generation_result =
                    self.generate_thumbnail_with_recovery(original_path, &thumbnail_path);

                match generation_result {
                    Ok(()) => {
                        // Update cache on success
                        {
                            let mut cache_guard = cache.lock().unwrap();
                            cache_guard.insert(asset.id.clone(), thumbnail_path);
                        }
                    }
                    Err(e) => {
                        error_count.fetch_add(1, Ordering::Relaxed);
                        log::warn!("Failed to generate thumbnail for {}: {}", asset.path, e);
                        // Continue processing other files instead of failing completely
                    }
                }

                // Update progress regardless of success/failure
                let current_count = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                self.send_progress_update(
                    current_count,
                    total_assets,
                    &asset.path,
                    start_time,
                    &progress_sender,
                );

                Ok(())
            })
            .collect();

        // Check if we were cancelled
        if self.cancellation_token.load(Ordering::Relaxed) {
            return Err(ThumbnailError::Cancelled);
        }

        // Count errors but don't fail the entire operation
        let total_errors = error_count.load(Ordering::Relaxed);
        let successful = total_assets - total_errors;

        // Send completion progress
        if let Some(sender) = &progress_sender {
            let completion_message = if total_errors > 0 {
                format!(
                    "Thumbnail generation complete: {} successful, {} failed",
                    successful, total_errors
                )
            } else {
                "Thumbnail generation complete".to_string()
            };

            let _ = sender.send(ThumbnailProgress {
                thumbnails_generated: total_assets,
                total_thumbnails: total_assets,
                current_file: completion_message,
                estimated_time_remaining: Some(0),
            });
        }

        log::info!(
            "Thumbnail generation completed: {} successful, {} failed out of {} total",
            successful,
            total_errors,
            total_assets
        );

        Ok(())
    }

    /// Generate a single thumbnail with error recovery
    fn generate_thumbnail_with_recovery(
        &self,
        original_path: &Path,
        thumbnail_path: &Path,
    ) -> Result<(), ThumbnailError> {
        // Validate input path
        if !original_path.exists() {
            return Err(ThumbnailError::InvalidPath {
                path: original_path.to_string_lossy().to_string(),
            });
        }

        // Create thumbnail directory if it doesn't exist
        if let Some(parent) = thumbnail_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try to load and process the image with multiple fallback strategies
        let img = match self.load_image_with_fallback(original_path) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load image {}: {}", original_path.display(), e);
                return Err(e);
            }
        };

        // Resize the image
        let thumbnail = self.resize_image(img, self.thumbnail_size)?;

        // Save thumbnail with error handling
        match self.save_thumbnail(&thumbnail, thumbnail_path) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::warn!(
                    "Failed to save thumbnail to {}: {}",
                    thumbnail_path.display(),
                    e
                );
                // Try to clean up partial file
                if thumbnail_path.exists() {
                    let _ = fs::remove_file(thumbnail_path);
                }
                Err(e)
            }
        }
    }

    /// Load image with multiple fallback strategies for corrupted files
    fn load_image_with_fallback(&self, path: &Path) -> Result<DynamicImage, ThumbnailError> {
        // First attempt: normal image loading
        match image::open(path) {
            Ok(img) => return Ok(img),
            Err(e) => {
                log::debug!("Primary image load failed for {}: {}", path.display(), e);
            }
        }

        // Second attempt: try to load with specific decoders
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();

            // Try format-specific loading for common problematic formats
            match ext_str.as_str() {
                "heic" | "heif" => {
                    // HEIC files might need special handling
                    log::debug!("Attempting HEIC-specific loading for {}", path.display());
                    // For now, fall through to generic error
                }
                "cr3" | "nef" | "arw" | "dng" => {
                    // RAW files might need special handling
                    log::debug!("RAW file detected: {}", path.display());
                    // For now, fall through to generic error
                }
                _ => {}
            }
        }

        // If all attempts fail, return appropriate error
        if let Some(ext) = path.extension() {
            Err(ThumbnailError::UnsupportedFormat {
                format: ext.to_string_lossy().to_string(),
            })
        } else {
            Err(ThumbnailError::Image(image::ImageError::Unsupported(
                image::error::UnsupportedError::from(image::error::ImageFormatHint::Unknown),
            )))
        }
    }

    /// Get the expected thumbnail path for an asset
    pub fn get_thumbnail_path(&self, project_temp_dir: &Path, asset_id: &str) -> PathBuf {
        project_temp_dir
            .join("thumbnails")
            .join(format!("{}.jpg", asset_id))
    }

    /// Get thumbnail path from cache if available
    pub fn get_cached_thumbnail_path(&self, asset_id: &str) -> Option<PathBuf> {
        let cache = self.cache.lock().unwrap();
        cache.get(asset_id).cloned()
    }

    /// Check if a thumbnail exists and is valid (newer than original)
    pub fn is_thumbnail_valid(
        &self,
        thumbnail_path: &Path,
        original_path: &Path,
    ) -> Result<bool, ThumbnailError> {
        if !thumbnail_path.exists() {
            return Ok(false);
        }

        // Check if thumbnail is newer than original
        let thumbnail_modified = fs::metadata(thumbnail_path)?.modified()?;
        let original_modified = fs::metadata(original_path)?.modified()?;

        Ok(thumbnail_modified >= original_modified)
    }

    /// Clean up all thumbnails for a project
    pub async fn cleanup_thumbnails(&self, project_temp_dir: &Path) -> Result<(), std::io::Error> {
        let thumbnails_dir = project_temp_dir.join("thumbnails");

        if thumbnails_dir.exists() {
            fs::remove_dir_all(&thumbnails_dir)?;
        }

        // Clear cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.clear();
        }

        Ok(())
    }

    /// Clean up thumbnails for specific assets
    pub async fn cleanup_asset_thumbnails(
        &self,
        project_temp_dir: &Path,
        asset_ids: &[String],
    ) -> Result<(), std::io::Error> {
        let mut cache = self.cache.lock().unwrap();

        for asset_id in asset_ids {
            let thumbnail_path = self.get_thumbnail_path(project_temp_dir, asset_id);

            if thumbnail_path.exists() {
                fs::remove_file(&thumbnail_path)?;
            }

            cache.remove(asset_id);
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let cached_count = cache.len();

        // Count how many cached thumbnails actually exist on disk
        let valid_count = cache.values().filter(|path| path.exists()).count();

        (cached_count, valid_count)
    }

    /// Clear invalid entries from cache
    pub fn cleanup_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, path| path.exists());
    }

    // Private helper methods

    fn load_image(&self, path: &Path) -> Result<DynamicImage, ThumbnailError> {
        // Try to load the image
        let img = image::open(path).map_err(|e| {
            // Check if it's an unsupported format error
            if let Some(ext) = path.extension() {
                ThumbnailError::UnsupportedFormat {
                    format: ext.to_string_lossy().to_string(),
                }
            } else {
                ThumbnailError::Image(e)
            }
        })?;

        Ok(img)
    }

    fn resize_image(
        &self,
        img: DynamicImage,
        target_size: u32,
    ) -> Result<DynamicImage, ThumbnailError> {
        let (width, height) = img.dimensions();

        // Calculate new dimensions maintaining aspect ratio
        let (new_width, new_height) = if width > height {
            let ratio = target_size as f32 / width as f32;
            (target_size, (height as f32 * ratio) as u32)
        } else {
            let ratio = target_size as f32 / height as f32;
            ((width as f32 * ratio) as u32, target_size)
        };

        // Use Lanczos3 filter for high-quality resizing
        let resized = img.resize(new_width, new_height, FilterType::Lanczos3);
        Ok(resized)
    }

    fn save_thumbnail(&self, img: &DynamicImage, path: &Path) -> Result<(), ThumbnailError> {
        // Convert to RGB if necessary (for JPEG output)
        let rgb_img = img.to_rgb8();

        // Save as JPEG with specified quality
        let mut output = fs::File::create(path)?;
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, self.quality);

        rgb_img.write_with_encoder(encoder)?;

        Ok(())
    }

    fn send_progress_update(
        &self,
        current: usize,
        total: usize,
        current_file: &str,
        start_time: std::time::Instant,
        progress_sender: &Option<mpsc::UnboundedSender<ThumbnailProgress>>,
    ) {
        if let Some(sender) = progress_sender {
            let elapsed = start_time.elapsed().as_secs();
            let estimated_remaining = if current > 0 && elapsed > 0 {
                let rate = current as f64 / elapsed as f64;
                let remaining_files = total - current;
                Some((remaining_files as f64 / rate) as u64)
            } else {
                None
            };

            let _ = sender.send(ThumbnailProgress {
                thumbnails_generated: current,
                total_thumbnails: total,
                current_file: current_file.to_string(),
                estimated_time_remaining: estimated_remaining,
            });
        }
    }
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::Asset;
    use tempfile::TempDir;

    fn create_test_asset(id: &str, path: &str) -> Asset {
        Asset {
            id: id.to_string(),
            project_id: "test_project".to_string(),
            path: path.to_string(),
            thumbnail_path: None,
            hash: None,
            perceptual_hash: None,
            size: 1000,
            width: 1920,
            height: 1080,
            exif_data: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

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
    async fn test_thumbnail_path_generation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();
        let project_temp_dir = temp_dir.path();

        let thumbnail_path = service.get_thumbnail_path(project_temp_dir, "ast_123");
        let expected_path = project_temp_dir.join("thumbnails").join("ast_123.jpg");

        assert_eq!(thumbnail_path, expected_path);
    }

    #[tokio::test]
    async fn test_single_thumbnail_generation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Create a test image
        let original_path = temp_dir.path().join("test.jpg");
        create_test_image(&original_path, 1920, 1080).unwrap();

        let thumbnail_path = temp_dir.path().join("thumbnail.jpg");

        // Generate thumbnail
        let result = service
            .generate_thumbnail(&original_path, &thumbnail_path)
            .await;
        assert!(result.is_ok());

        // Verify thumbnail exists
        assert!(thumbnail_path.exists());

        // Verify thumbnail dimensions
        let thumbnail_img = image::open(&thumbnail_path).unwrap();
        let (width, height) = thumbnail_img.dimensions();
        assert!(width <= 512 && height <= 512);
        assert!(width == 512 || height == 512); // One dimension should be exactly 512
    }

    #[tokio::test]
    async fn test_batch_thumbnail_generation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();
        let project_temp_dir = temp_dir.path().join("project");

        // Create test images and assets
        let mut assets = Vec::new();
        for i in 0..3 {
            let original_path = temp_dir.path().join(format!("test_{}.jpg", i));
            create_test_image(&original_path, 1920, 1080).unwrap();

            let asset = create_test_asset(&format!("ast_{}", i), &original_path.to_string_lossy());
            assets.push(asset);
        }

        // Generate thumbnails
        let result = service
            .generate_thumbnails_batch(&assets, &project_temp_dir, None)
            .await;
        if let Err(e) = &result {
            println!("Error generating thumbnails: {:?}", e);
        }
        assert!(result.is_ok());

        // Verify all thumbnails exist
        for asset in &assets {
            let thumbnail_path = service.get_thumbnail_path(&project_temp_dir, &asset.id);
            assert!(thumbnail_path.exists());
        }

        // Verify cache is populated
        let (cached_count, valid_count) = service.get_cache_stats();
        assert_eq!(cached_count, 3);
        assert_eq!(valid_count, 3);
    }

    #[tokio::test]
    async fn test_thumbnail_validation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Create original and thumbnail files
        let original_path = temp_dir.path().join("original.jpg");
        let thumbnail_path = temp_dir.path().join("thumbnail.jpg");

        create_test_image(&original_path, 1920, 1080).unwrap();
        create_test_image(&thumbnail_path, 512, 512).unwrap();

        // Thumbnail should be valid (both exist)
        let is_valid = service
            .is_thumbnail_valid(&thumbnail_path, &original_path)
            .unwrap();
        assert!(is_valid);

        // Non-existent thumbnail should be invalid
        let missing_thumbnail = temp_dir.path().join("missing.jpg");
        let is_valid = service
            .is_thumbnail_valid(&missing_thumbnail, &original_path)
            .unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_thumbnail_cleanup() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();
        let project_temp_dir = temp_dir.path().join("project");

        // Create test asset and generate thumbnail
        let original_path = temp_dir.path().join("test.jpg");
        create_test_image(&original_path, 1920, 1080).unwrap();

        let asset = create_test_asset("ast_123", &original_path.to_string_lossy());
        let result = service
            .generate_thumbnails_batch(&[asset], &project_temp_dir, None)
            .await;
        assert!(result.is_ok());

        // Verify thumbnail exists
        let thumbnail_path = service.get_thumbnail_path(&project_temp_dir, "ast_123");
        assert!(thumbnail_path.exists());

        // Clean up thumbnails
        let result = service.cleanup_thumbnails(&project_temp_dir).await;
        if let Err(e) = &result {
            println!("Error cleaning up thumbnails: {:?}", e);
        }
        assert!(result.is_ok());

        // Verify thumbnail is removed
        assert!(!thumbnail_path.exists());

        // Verify cache is cleared
        let (cached_count, _) = service.get_cache_stats();
        assert_eq!(cached_count, 0);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();
        let project_temp_dir = temp_dir.path().join("project");

        // Create many test assets
        let mut assets = Vec::new();
        for i in 0..10 {
            let original_path = temp_dir.path().join(format!("test_{}.jpg", i));
            create_test_image(&original_path, 1920, 1080).unwrap();

            let asset = create_test_asset(&format!("ast_{}", i), &original_path.to_string_lossy());
            assets.push(asset);
        }

        // Cancel before starting
        service.cancel_generation();

        // Try to generate thumbnails
        let result = service
            .generate_thumbnails_batch(&assets, &project_temp_dir, None)
            .await;
        assert!(matches!(result, Err(ThumbnailError::Cancelled)));
    }

    #[tokio::test]
    async fn test_aspect_ratio_preservation() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Test with wide image (landscape)
        let wide_original = temp_dir.path().join("wide.jpg");
        create_test_image(&wide_original, 1920, 1080).unwrap();

        let wide_thumbnail = temp_dir.path().join("wide_thumb.jpg");
        service
            .generate_thumbnail(&wide_original, &wide_thumbnail)
            .await
            .unwrap();

        let wide_img = image::open(&wide_thumbnail).unwrap();
        let (w_width, w_height) = wide_img.dimensions();
        assert_eq!(w_width, 512); // Width should be 512 for landscape
        assert!(w_height < 512); // Height should be proportionally smaller

        // Test with tall image (portrait)
        let tall_original = temp_dir.path().join("tall.jpg");
        create_test_image(&tall_original, 1080, 1920).unwrap();

        let tall_thumbnail = temp_dir.path().join("tall_thumb.jpg");
        service
            .generate_thumbnail(&tall_original, &tall_thumbnail)
            .await
            .unwrap();

        let tall_img = image::open(&tall_thumbnail).unwrap();
        let (t_width, t_height) = tall_img.dimensions();
        assert_eq!(t_height, 512); // Height should be 512 for portrait
        assert!(t_width < 512); // Width should be proportionally smaller
    }
}
