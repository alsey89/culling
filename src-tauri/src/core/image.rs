use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub created_at: Option<String>,
    pub modified_at: String,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub iso: Option<u32>,
    pub exposure_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageHash {
    pub path: String,
    pub md5_hash: String,
    pub perceptual_hash: String,
    pub file_size: u64,
}

impl ImageMetadata {
    pub fn from_path(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path_buf = Path::new(path);
        
        // Get basic file metadata
        let file_metadata = std::fs::metadata(path)?;
        let filename = path_buf
            .file_name()
            .ok_or("Invalid filename")?
            .to_string_lossy()
            .to_string();
        
        let modified_at = file_metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // For now, we'll create a basic implementation
        // In a production app, you'd use libraries like `image` or `exif` crates
        // to extract proper image metadata and EXIF data
        
        // Try to determine format from extension
        let format = path_buf
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_uppercase();
        
        // Basic image info extraction (placeholder)
        // In reality, you'd use the `image` crate to load and inspect the image
        let (width, height) = match Self::get_image_dimensions(path) {
            Ok((w, h)) => (w, h),
            Err(_) => (0, 0), // Fallback for unsupported formats
        };
        
        Ok(ImageMetadata {
            path: path.to_string(),
            filename,
            size_bytes: file_metadata.len(),
            width,
            height,
            format,
            created_at: None, // Would extract from EXIF
            modified_at: chrono::DateTime::from_timestamp(modified_at as i64, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string()),
            camera_make: None,
            camera_model: None,
            focal_length: None,
            aperture: None,
            iso: None,
            exposure_time: None,
        })
    }
    
    fn get_image_dimensions(path: &str) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        // This is a placeholder implementation
        // In a real app, you'd use the `image` crate to load the image and get dimensions
        // For now, we'll return dummy values
        
        // Check if it's a supported format
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".jpg") || 
           path_lower.ends_with(".jpeg") || 
           path_lower.ends_with(".png") ||
           path_lower.ends_with(".tiff") ||
           path_lower.ends_with(".tif") ||
           path_lower.ends_with(".bmp") ||
           path_lower.ends_with(".gif") ||
           path_lower.ends_with(".webp") {
            // Return placeholder dimensions
            // In reality: let img = image::open(path)?; return (img.width(), img.height());
            Ok((1920, 1080)) // Placeholder
        } else {
            Err("Unsupported image format".into())
        }
    }
}

impl ImageHash {
    pub async fn compute(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Read file for MD5 hash
        let file_data = fs::read(path)?;
        let file_size = file_data.len() as u64;
        
        // Compute MD5 hash
        let md5_hash = format!("{:x}", md5::compute(&file_data));
        
        // Placeholder for perceptual hash
        // In a real implementation, you'd use a library like `img_hash` 
        // to compute perceptual hashes for duplicate detection
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        let perceptual_hash = format!("{:x}", hasher.finish());
        
        Ok(ImageHash {
            path: path.to_string(),
            md5_hash,
            perceptual_hash,
            file_size,
        })
    }
}
