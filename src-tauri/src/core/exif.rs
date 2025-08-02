use crate::database::models::ExifData;
use chrono::{DateTime, Utc};
use exif::{In, Reader, Tag, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExifError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("EXIF parsing error: {0}")]
    Parse(#[from] exif::Error),

    #[error("Date parsing error: {message}")]
    DateParse { message: String },
}

/// Service for extracting EXIF metadata from image files
pub struct ExifService;

impl ExifService {
    pub fn new() -> Self {
        Self
    }

    /// Extract EXIF data from an image file
    pub fn extract_exif(&self, file_path: &Path) -> Result<Option<ExifData>, ExifError> {
        // Try to open and read the file
        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(_) => return Ok(None), // File not readable, return None instead of error
        };

        let mut buf_reader = BufReader::new(file);

        // Try to parse EXIF data
        let exif_reader = match Reader::new().read_from_container(&mut buf_reader) {
            Ok(reader) => reader,
            Err(_) => return Ok(None), // No EXIF data or unsupported format
        };

        let mut exif_data = ExifData {
            taken_at: None,
            camera: None,
            lens: None,
            iso: None,
            aperture: None,
            shutter_speed: None,
        };

        // Extract date/time taken
        if let Some(field) = exif_reader.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            if let Some(datetime_str) = self.field_to_string(&field.value) {
                exif_data.taken_at = self.parse_exif_datetime(&datetime_str);
            }
        } else if let Some(field) = exif_reader.get_field(Tag::DateTime, In::PRIMARY) {
            if let Some(datetime_str) = self.field_to_string(&field.value) {
                exif_data.taken_at = self.parse_exif_datetime(&datetime_str);
            }
        }

        // Extract camera make and model
        let mut camera_parts = Vec::new();
        if let Some(field) = exif_reader.get_field(Tag::Make, In::PRIMARY) {
            if let Some(make) = self.field_to_string(&field.value) {
                camera_parts.push(make.trim().to_string());
            }
        }
        if let Some(field) = exif_reader.get_field(Tag::Model, In::PRIMARY) {
            if let Some(model) = self.field_to_string(&field.value) {
                camera_parts.push(model.trim().to_string());
            }
        }
        if !camera_parts.is_empty() {
            exif_data.camera = Some(camera_parts.join(" "));
        }

        // Extract lens information
        if let Some(field) = exif_reader.get_field(Tag::LensModel, In::PRIMARY) {
            exif_data.lens = self.field_to_string(&field.value);
        } else if let Some(field) = exif_reader.get_field(Tag::LensMake, In::PRIMARY) {
            exif_data.lens = self.field_to_string(&field.value);
        }

        // Extract ISO
        if let Some(field) = exif_reader.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
            exif_data.iso = self.field_to_u32(&field.value);
        } else if let Some(field) = exif_reader.get_field(Tag::ISOSpeed, In::PRIMARY) {
            exif_data.iso = self.field_to_u32(&field.value);
        }

        // Extract aperture (F-number)
        if let Some(field) = exif_reader.get_field(Tag::FNumber, In::PRIMARY) {
            exif_data.aperture = self.field_to_f32(&field.value);
        }

        // Extract shutter speed (exposure time)
        if let Some(field) = exif_reader.get_field(Tag::ExposureTime, In::PRIMARY) {
            exif_data.shutter_speed = self.field_to_string(&field.value);
        }

        // Return Some(exif_data) if we extracted any meaningful data
        if exif_data.taken_at.is_some()
            || exif_data.camera.is_some()
            || exif_data.lens.is_some()
            || exif_data.iso.is_some()
            || exif_data.aperture.is_some()
            || exif_data.shutter_speed.is_some()
        {
            Ok(Some(exif_data))
        } else {
            Ok(None)
        }
    }

    /// Extract EXIF data from multiple files in parallel
    pub fn extract_exif_batch(
        &self,
        file_paths: &[&Path],
    ) -> Vec<(String, Result<Option<ExifData>, ExifError>)> {
        use rayon::prelude::*;

        file_paths
            .par_iter()
            .map(|path| {
                let path_str = path.to_string_lossy().to_string();
                let exif_result = self.extract_exif(path);
                (path_str, exif_result)
            })
            .collect()
    }

    /// Convert EXIF field value to string
    fn field_to_string(&self, value: &Value) -> Option<String> {
        match value {
            Value::Ascii(vec) => {
                if let Some(ascii_val) = vec.first() {
                    Some(
                        String::from_utf8_lossy(ascii_val)
                            .trim_end_matches('\0')
                            .to_string(),
                    )
                } else {
                    None
                }
            }
            Value::Undefined(data, _) => Some(
                String::from_utf8_lossy(data)
                    .trim_end_matches('\0')
                    .to_string(),
            ),
            _ => Some(format!("{}", value.display_as(Tag::DateTime))),
        }
    }

    /// Convert EXIF field value to u32
    fn field_to_u32(&self, value: &Value) -> Option<u32> {
        match value {
            Value::Short(vec) => vec.first().map(|&v| v as u32),
            Value::Long(vec) => vec.first().copied(),
            Value::Ascii(vec) => {
                if let Some(ascii_val) = vec.first() {
                    let s = String::from_utf8_lossy(ascii_val)
                        .trim_end_matches('\0')
                        .to_string();
                    s.parse().ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert EXIF field value to f32
    fn field_to_f32(&self, value: &Value) -> Option<f32> {
        match value {
            Value::Rational(vec) => {
                if let Some(rational) = vec.first() {
                    if rational.denom != 0 {
                        Some(rational.num as f32 / rational.denom as f32)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Value::SRational(vec) => {
                if let Some(rational) = vec.first() {
                    if rational.denom != 0 {
                        Some(rational.num as f32 / rational.denom as f32)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Parse EXIF datetime string to UTC DateTime
    fn parse_exif_datetime(&self, datetime_str: &str) -> Option<DateTime<Utc>> {
        // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
        let normalized = datetime_str.replace(':', "-");

        // Try different parsing approaches
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S") {
            Some(dt.and_utc())
        } else if let Ok(dt) =
            chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y:%m:%d %H:%M:%S")
        {
            Some(dt.and_utc())
        } else {
            log::warn!("Failed to parse EXIF datetime: {}", datetime_str);
            None
        }
    }
}

impl Default for ExifService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_exif_no_file() {
        let exif_service = ExifService::new();
        let non_existent = Path::new("/non/existent/file.jpg");

        let result = exif_service.extract_exif(non_existent);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_extract_exif_no_exif_data() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("no_exif.txt");

        // Create a text file with no EXIF data
        fs::write(&file_path, b"This is not an image file").unwrap();

        let exif_service = ExifService::new();
        let result = exif_service.extract_exif(&file_path);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_exif_datetime() {
        use chrono::{Datelike, Timelike};

        let exif_service = ExifService::new();

        // Test standard EXIF datetime format
        let dt = exif_service.parse_exif_datetime("2023:12:25 14:30:45");
        assert!(dt.is_some());

        let parsed = dt.unwrap();
        assert_eq!(parsed.year(), 2023);
        assert_eq!(parsed.month(), 12);
        assert_eq!(parsed.day(), 25);
        assert_eq!(parsed.hour(), 14);
        assert_eq!(parsed.minute(), 30);
        assert_eq!(parsed.second(), 45);
    }

    #[test]
    fn test_batch_exif_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"File 1 content").unwrap();
        fs::write(&file2, b"File 2 content").unwrap();

        let exif_service = ExifService::new();
        let paths = vec![file1.as_path(), file2.as_path()];
        let results = exif_service.extract_exif_batch(&paths);

        assert_eq!(results.len(), 2);
        assert!(results[0].1.is_ok());
        assert!(results[1].1.is_ok());

        // Both should return None since they're not image files
        assert!(results[0].1.as_ref().unwrap().is_none());
        assert!(results[1].1.as_ref().unwrap().is_none());
    }
}
