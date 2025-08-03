use image::{imageops::FilterType, DynamicImage, GenericImageView};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
}

#[derive(Debug, Clone, Serialize)]
pub struct ThumbnailProgress {
    pub current_file: String,
    pub completed_count: usize,
    pub total_count: usize,
    pub current_phase: ThumbnailPhase,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ThumbnailPhase {
    Loading,
    Processing,
    Saving,
    Complete,
    Error,
}

pub type ProgressCallback = Box<dyn Fn(ThumbnailProgress) + Send + Sync>;

pub struct ThumbnailService {
    thumbnail_size: u32,
    quality: u8,
}

impl ThumbnailService {
    pub fn new() -> Self {
        Self {
            thumbnail_size: 512,
            quality: 85,
        }
    }

    /// Generate a single thumbnail from an original image file
    pub async fn generate_thumbnail(
        &self,
        original_path: &Path,
        thumbnail_path: &Path,
    ) -> Result<(), ThumbnailError> {
        self.generate_thumbnail_with_progress(original_path, thumbnail_path, None, 0, 1)
            .await
    }

    /// Generate a single thumbnail with progress reporting
    pub async fn generate_thumbnail_with_progress(
        &self,
        original_path: &Path,
        thumbnail_path: &Path,
        progress_callback: Option<&ProgressCallback>,
        current_index: usize,
        total_count: usize,
    ) -> Result<(), ThumbnailError> {
        let file_name = original_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Helper function to safely call progress callback
        let report_progress = |phase: ThumbnailPhase, error_message: Option<String>| {
            if let Some(callback) = progress_callback {
                let progress = ThumbnailProgress {
                    current_file: file_name.clone(),
                    completed_count: if matches!(phase, ThumbnailPhase::Complete) {
                        current_index + 1
                    } else {
                        current_index
                    },
                    total_count,
                    current_phase: phase,
                    error_message,
                };

                // Safely call callback, log errors but don't fail thumbnail generation
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(progress);
                })) {
                    log::warn!("Progress callback panicked: {:?}", e);
                }
            }
        };

        // Validate input path
        if !original_path.exists() {
            let error = ThumbnailError::InvalidPath {
                path: original_path.to_string_lossy().to_string(),
            };
            report_progress(
                ThumbnailPhase::Error,
                Some(format!("File not found: {}", original_path.display())),
            );
            return Err(error);
        }

        // Create thumbnail directory if it doesn't exist
        if let Some(parent) = thumbnail_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                report_progress(
                    ThumbnailPhase::Error,
                    Some(format!("Failed to create directory: {}", e)),
                );
                return Err(ThumbnailError::Io(e));
            }
        }

        // Report loading phase
        report_progress(ThumbnailPhase::Loading, None);

        // Load and process the image
        let img = match self.load_image(original_path) {
            Ok(img) => img,
            Err(e) => {
                report_progress(
                    ThumbnailPhase::Error,
                    Some(format!("Failed to load image: {}", e)),
                );
                return Err(e);
            }
        };

        // Report processing phase
        report_progress(ThumbnailPhase::Processing, None);

        let thumbnail = match self.resize_image(img, self.thumbnail_size) {
            Ok(thumbnail) => thumbnail,
            Err(e) => {
                report_progress(
                    ThumbnailPhase::Error,
                    Some(format!("Failed to resize image: {}", e)),
                );
                return Err(e);
            }
        };

        // Report saving phase
        report_progress(ThumbnailPhase::Saving, None);

        // Save thumbnail with JPEG format for consistent size and quality
        match self.save_thumbnail(&thumbnail, thumbnail_path) {
            Ok(()) => {
                report_progress(ThumbnailPhase::Complete, None);
                Ok(())
            }
            Err(e) => {
                report_progress(
                    ThumbnailPhase::Error,
                    Some(format!("Failed to save thumbnail: {}", e)),
                );
                Err(e)
            }
        }
    }

    /// Get the expected thumbnail path for an asset
    pub fn get_thumbnail_path(&self, project_temp_dir: &Path, asset_id: &str) -> PathBuf {
        project_temp_dir
            .join("thumbnails")
            .join(format!("{}.jpg", asset_id))
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
}

impl Default for ThumbnailService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_progress_reporting() {
        use std::sync::{Arc, Mutex};

        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Create a test image
        let original_path = temp_dir.path().join("test.jpg");
        create_test_image(&original_path, 1920, 1080).unwrap();

        let thumbnail_path = temp_dir.path().join("thumbnail.jpg");

        // Track progress updates
        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_updates_clone = progress_updates.clone();

        let callback: ProgressCallback = Box::new(move |progress| {
            progress_updates_clone.lock().unwrap().push(progress);
        });

        // Generate thumbnail with progress reporting
        let result = service
            .generate_thumbnail_with_progress(
                &original_path,
                &thumbnail_path,
                Some(&callback),
                0,
                1,
            )
            .await;

        assert!(result.is_ok());
        assert!(thumbnail_path.exists());

        // Verify progress updates were received
        let updates = progress_updates.lock().unwrap();
        assert!(!updates.is_empty());

        // Should have at least Loading, Processing, Saving, Complete phases
        let phases: Vec<_> = updates.iter().map(|p| &p.current_phase).collect();
        assert!(phases.contains(&&ThumbnailPhase::Loading));
        assert!(phases.contains(&&ThumbnailPhase::Processing));
        assert!(phases.contains(&&ThumbnailPhase::Saving));
        assert!(phases.contains(&&ThumbnailPhase::Complete));

        // Verify final progress shows completion
        let final_progress = updates.last().unwrap();
        assert!(matches!(
            final_progress.current_phase,
            ThumbnailPhase::Complete
        ));
        assert_eq!(final_progress.completed_count, 1);
        assert_eq!(final_progress.total_count, 1);
        assert!(final_progress.error_message.is_none());
    }

    #[tokio::test]
    async fn test_progress_error_handling() {
        use std::sync::{Arc, Mutex};

        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Use non-existent file to trigger error
        let original_path = temp_dir.path().join("nonexistent.jpg");
        let thumbnail_path = temp_dir.path().join("thumbnail.jpg");

        // Track progress updates
        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_updates_clone = progress_updates.clone();

        let callback: ProgressCallback = Box::new(move |progress| {
            progress_updates_clone.lock().unwrap().push(progress);
        });

        // Generate thumbnail with progress reporting (should fail)
        let result = service
            .generate_thumbnail_with_progress(
                &original_path,
                &thumbnail_path,
                Some(&callback),
                0,
                1,
            )
            .await;

        assert!(result.is_err());

        // Verify error was reported via progress
        let updates = progress_updates.lock().unwrap();
        assert!(!updates.is_empty());

        let final_progress = updates.last().unwrap();
        assert!(matches!(
            final_progress.current_phase,
            ThumbnailPhase::Error
        ));
        assert!(final_progress.error_message.is_some());
    }

    #[tokio::test]
    async fn test_callback_error_handling() {
        let service = ThumbnailService::new();
        let temp_dir = TempDir::new().unwrap();

        // Create a test image
        let original_path = temp_dir.path().join("test.jpg");
        create_test_image(&original_path, 1920, 1080).unwrap();

        let thumbnail_path = temp_dir.path().join("thumbnail.jpg");

        // Create a callback that panics
        let callback: ProgressCallback = Box::new(move |_progress| {
            panic!("Test panic in callback");
        });

        // Generate thumbnail with panicking callback - should still succeed
        let result = service
            .generate_thumbnail_with_progress(
                &original_path,
                &thumbnail_path,
                Some(&callback),
                0,
                1,
            )
            .await;

        // Thumbnail generation should succeed despite callback panic
        assert!(result.is_ok());
        assert!(thumbnail_path.exists());
    }
}
