use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageFile {
    pub path: PathBuf,
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub format: ImageFormat,
    pub dimensions: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Tiff,
    Raw,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub recursive: bool,
    pub max_depth: Option<usize>,
    pub supported_formats: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            max_depth: None,
            supported_formats: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "tiff".to_string(),
                "tif".to_string(),
                "raw".to_string(),
                "cr2".to_string(),
                "nef".to_string(),
                "arw".to_string(),
            ],
        }
    }
}

pub struct ScannerService;

impl ScannerService {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan_directory(
        &self,
        path: PathBuf,
        options: ScanOptions,
    ) -> Result<mpsc::Receiver<Result<ImageFile>>> {
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            let walker = if let Some(max_depth) = options.max_depth {
                WalkDir::new(&path).max_depth(max_depth)
            } else {
                WalkDir::new(&path)
            };

            for entry in walker {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().is_file() {
                            if let Some(extension) = entry.path().extension() {
                                let ext = extension.to_string_lossy().to_lowercase();
                                if options.supported_formats.contains(&ext) {
                                    match Self::create_image_file(entry.path()) {
                                        Ok(image_file) => {
                                            if tx.send(Ok(image_file)).await.is_err() {
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            if tx.send(Err(e)).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if tx.send(Err(anyhow::anyhow!("Walk error: {}", e))).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn create_image_file(path: &std::path::Path) -> Result<ImageFile> {
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata.modified()?;
        
        let format = Self::detect_format(path);
        
        Ok(ImageFile {
            path: path.to_path_buf(),
            size,
            modified,
            format,
            dimensions: None, // Will be populated later when needed
        })
    }

    fn detect_format(path: &std::path::Path) -> ImageFormat {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "jpg" | "jpeg" => ImageFormat::Jpeg,
                "png" => ImageFormat::Png,
                "tiff" | "tif" => ImageFormat::Tiff,
                "raw" | "cr2" | "nef" | "arw" => ImageFormat::Raw,
                other => ImageFormat::Other(other.to_string()),
            }
        } else {
            ImageFormat::Other("unknown".to_string())
        }
    }
}