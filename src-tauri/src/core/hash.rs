use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HashError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hash computation failed: {message}")]
    ComputationFailed { message: String },
}

/// Service for computing various types of hashes for images
pub struct HashService;

impl HashService {
    pub fn new() -> Self {
        Self
    }

    /// Compute SHA-256 content hash from original file
    /// This is used for exact duplicate detection
    pub fn compute_content_hash(&self, file_path: &Path) -> Result<String, HashError> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192]; // 8KB buffer for efficient reading

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Compute content hashes for multiple files in parallel
    /// Returns a vector of (file_path, hash) tuples
    pub fn compute_content_hashes_batch(
        &self,
        file_paths: &[&Path],
    ) -> Vec<(String, Result<String, HashError>)> {
        use rayon::prelude::*;

        file_paths
            .par_iter()
            .map(|path| {
                let path_str = path.to_string_lossy().to_string();
                let hash_result = self.compute_content_hash(path);
                (path_str, hash_result)
            })
            .collect()
    }

    /// Verify if two files have the same content hash
    pub fn verify_identical_content(&self, file1: &Path, file2: &Path) -> Result<bool, HashError> {
        let hash1 = self.compute_content_hash(file1)?;
        let hash2 = self.compute_content_hash(file2)?;
        Ok(hash1 == hash2)
    }
}

impl Default for HashService {
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
    fn test_compute_content_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with known content
        let content = b"Hello, World!";
        fs::write(&file_path, content).unwrap();

        let hash_service = HashService::new();
        let hash = hash_service.compute_content_hash(&file_path).unwrap();

        // Verify hash is consistent
        let hash2 = hash_service.compute_content_hash(&file_path).unwrap();
        assert_eq!(hash, hash2);

        // Verify hash format (64 hex characters for SHA-256)
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_identical_files_same_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        let content = b"Identical content";
        fs::write(&file1, content).unwrap();
        fs::write(&file2, content).unwrap();

        let hash_service = HashService::new();
        let hash1 = hash_service.compute_content_hash(&file1).unwrap();
        let hash2 = hash_service.compute_content_hash(&file2).unwrap();

        assert_eq!(hash1, hash2);
        assert!(hash_service
            .verify_identical_content(&file1, &file2)
            .unwrap());
    }

    #[test]
    fn test_different_files_different_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"Content A").unwrap();
        fs::write(&file2, b"Content B").unwrap();

        let hash_service = HashService::new();
        let hash1 = hash_service.compute_content_hash(&file1).unwrap();
        let hash2 = hash_service.compute_content_hash(&file2).unwrap();

        assert_ne!(hash1, hash2);
        assert!(!hash_service
            .verify_identical_content(&file1, &file2)
            .unwrap());
    }

    #[test]
    fn test_batch_hashing() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"Content 1").unwrap();
        fs::write(&file2, b"Content 2").unwrap();

        let hash_service = HashService::new();
        let paths = vec![file1.as_path(), file2.as_path()];
        let results = hash_service.compute_content_hashes_batch(&paths);

        assert_eq!(results.len(), 2);
        assert!(results[0].1.is_ok());
        assert!(results[1].1.is_ok());

        // Verify hashes are different
        let hash1 = results[0].1.as_ref().unwrap();
        let hash2 = results[1].1.as_ref().unwrap();
        assert_ne!(hash1, hash2);
    }
}
