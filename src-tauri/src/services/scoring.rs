use std::path::Path;
use anyhow::Result;
use image::{DynamicImage, ImageBuffer, Luma, GenericImageView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub overall: f64,
    pub sharpness: f64,
    pub exposure: f64,
    pub composition: f64,
    pub technical_issues: Vec<String>,
}

pub struct ScoringService;

impl ScoringService {
    pub fn new() -> Self {
        Self
    }

    pub async fn score_image<P: AsRef<Path>>(&self, image_path: P) -> Result<QualityScore> {
        let image = image::open(image_path.as_ref())?;
        self.score_image_from_dynamic(&image)
    }

    pub fn score_image_from_dynamic(&self, image: &DynamicImage) -> Result<QualityScore> {
        let sharpness = self.calculate_sharpness(image)?;
        let exposure = self.calculate_exposure(image)?;
        let composition = self.calculate_composition(image)?;
        
        let mut technical_issues = Vec::new();
        
        // Check for technical issues
        if sharpness < 0.3 {
            technical_issues.push("Image appears blurry".to_string());
        }
        if exposure < 0.2 || exposure > 0.8 {
            technical_issues.push("Poor exposure detected".to_string());
        }

        // Calculate overall score as weighted average
        let overall = (sharpness * 0.4) + (exposure * 0.3) + (composition * 0.3);

        Ok(QualityScore {
            overall,
            sharpness,
            exposure,
            composition,
            technical_issues,
        })
    }

    fn calculate_sharpness(&self, image: &DynamicImage) -> Result<f64> {
        // Convert to grayscale for sharpness analysis
        let gray_image = image.to_luma8();
        
        // Apply Laplacian filter to detect edges
        let laplacian_variance = self.laplacian_variance(&gray_image);
        
        // Normalize the variance to a 0-1 score
        // This is a simplified approach - in practice, you'd calibrate against known sharp/blurry images
        let normalized_score = (laplacian_variance / 1000.0).min(1.0);
        
        Ok(normalized_score)
    }

    fn laplacian_variance(&self, image: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f64 {
        let (width, height) = image.dimensions();
        let mut sum = 0.0;
        let mut count = 0;

        // Laplacian kernel
        let kernel = [
            [0, -1, 0],
            [-1, 4, -1],
            [0, -1, 0],
        ];

        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let mut laplacian = 0.0;
                
                for ky in 0..3 {
                    for kx in 0..3 {
                        let pixel_x = x + kx - 1;
                        let pixel_y = y + ky - 1;
                        let pixel_value = image.get_pixel(pixel_x, pixel_y)[0] as f64;
                        laplacian += pixel_value * kernel[ky as usize][kx as usize] as f64;
                    }
                }
                
                sum += laplacian * laplacian;
                count += 1;
            }
        }

        if count > 0 {
            sum / count as f64
        } else {
            0.0
        }
    }

    fn calculate_exposure(&self, image: &DynamicImage) -> Result<f64> {
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();
        
        let mut brightness_sum = 0.0;
        let total_pixels = (width * height) as f64;

        for pixel in rgb_image.pixels() {
            // Calculate luminance using standard weights
            let luminance = (0.299 * pixel[0] as f64) + 
                           (0.587 * pixel[1] as f64) + 
                           (0.114 * pixel[2] as f64);
            brightness_sum += luminance;
        }

        let average_brightness = brightness_sum / total_pixels;
        
        // Normalize to 0-1 range and apply exposure scoring
        let normalized_brightness = average_brightness / 255.0;
        
        // Score based on how close to optimal exposure (around 0.5)
        let exposure_score = 1.0 - (normalized_brightness - 0.5).abs() * 2.0;
        
        Ok(exposure_score.max(0.0))
    }

    fn calculate_composition(&self, image: &DynamicImage) -> Result<f64> {
        // Simplified composition analysis
        // In a real implementation, this would include rule of thirds, symmetry, etc.
        
        let (width, height) = image.dimensions();
        let aspect_ratio = width as f64 / height as f64;
        
        // Score based on common "good" aspect ratios
        let aspect_score = match aspect_ratio {
            r if (r - 1.618).abs() < 0.1 => 1.0, // Golden ratio
            r if (r - 1.5).abs() < 0.1 => 0.9,   // 3:2
            r if (r - 1.333).abs() < 0.1 => 0.8, // 4:3
            r if (r - 1.0).abs() < 0.1 => 0.7,   // Square
            _ => 0.5, // Other ratios
        };

        // This is a placeholder - real composition analysis would be much more complex
        Ok(aspect_score)
    }
}