use std::path::Path;
use anyhow::Result;
use sha2::{Sha256, Digest};
use memmap2::Mmap;
use std::fs::File;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHash(pub String);

impl FileHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct HashService;

impl HashService {
    pub fn new() -> Self {
        Self
    }

    pub async fn compute_hash<P: AsRef<Path>>(&self, file_path: P) -> Result<FileHash> {
        let path = file_path.as_ref();
        
        // Use memory-mapped file for efficient reading of large files
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&mmap);
        let result = hasher.finalize();
        
        let hash_string = format!("{:x}", result);
        Ok(FileHash(hash_string))
    }

    pub async fn compute_hash_batch(
        &self,
        file_paths: Vec<String>,
    ) -> Vec<Result<(String, FileHash)>> {
        use rayon::prelude::*;
        
        file_paths
            .into_par_iter()
            .map(|path_str| {
                let path = std::path::Path::new(&path_str);
                match self.compute_hash_sync(path) {
                    Ok(hash) => Ok((path_str, hash)),
                    Err(e) => Err(e),
                }
            })
            .collect()
    }

    fn compute_hash_sync<P: AsRef<Path>>(&self, file_path: P) -> Result<FileHash> {
        let path = file_path.as_ref();
        
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        let mut hasher = Sha256::new();
        hasher.update(&mmap);
        let result = hasher.finalize();
        
        let hash_string = format!("{:x}", result);
        Ok(FileHash(hash_string))
    }
}