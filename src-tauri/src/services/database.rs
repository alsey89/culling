use crate::services::hash::FileHash;
use crate::services::perceptual::PerceptualHash;
use crate::services::scanner::ImageFile;
use crate::services::scoring::QualityScore;
use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub group_id: Uuid,
    pub duplicate_type: DuplicateType,
    pub files: Vec<ImageFile>,
    pub similarity_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DuplicateType {
    Exact,
    NearDuplicate,
    Crop,
    Rotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub similarity_threshold: f64,
    pub supported_formats: Vec<String>,
    pub max_scan_depth: Option<usize>,
    pub parallel_workers: usize,
    pub cache_enabled: bool,
    pub ai_features_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            supported_formats: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "tiff".to_string(),
                "tif".to_string(),
            ],
            max_scan_depth: None,
            parallel_workers: num_cpus::get(),
            cache_enabled: true,
            ai_features_enabled: false,
        }
    }
}

pub struct DatabaseService {
    sqlite_conn: Connection,
    sled_db: Db,
}

impl DatabaseService {
    pub fn new(db_path: PathBuf, cache_path: PathBuf) -> Result<Self> {
        let sqlite_conn = Connection::open(db_path)?;
        let sled_db = sled::open(cache_path)?;

        let service = Self {
            sqlite_conn,
            sled_db,
        };

        service.initialize_schema()?;
        Ok(service)
    }

    fn initialize_schema(&self) -> Result<()> {
        // Create tables for file metadata and scan results
        self.sqlite_conn.execute(
            "CREATE TABLE IF NOT EXISTS scanned_files (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                size INTEGER NOT NULL,
                modified_time INTEGER NOT NULL,
                file_hash TEXT,
                perceptual_hash TEXT,
                quality_score REAL,
                scan_timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for performance
        self.sqlite_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_hash ON scanned_files(file_hash)",
            [],
        )?;

        self.sqlite_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_perceptual_hash ON scanned_files(perceptual_hash)",
            [],
        )?;

        // Create duplicate groups table
        self.sqlite_conn.execute(
            "CREATE TABLE IF NOT EXISTS duplicate_groups (
                id INTEGER PRIMARY KEY,
                group_type TEXT NOT NULL,
                similarity_score REAL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create group members table
        self.sqlite_conn.execute(
            "CREATE TABLE IF NOT EXISTS group_members (
                group_id INTEGER REFERENCES duplicate_groups(id),
                file_id INTEGER REFERENCES scanned_files(id),
                PRIMARY KEY (group_id, file_id)
            )",
            [],
        )?;

        // Create configuration table
        self.sqlite_conn.execute(
            "CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create scan sessions table
        self.sqlite_conn.execute(
            "CREATE TABLE IF NOT EXISTS scan_sessions (
                id INTEGER PRIMARY KEY,
                directory_path TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                files_processed INTEGER DEFAULT 0,
                status TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn store_scanned_file(&self, file: &ImageFile, hash: Option<&FileHash>) -> Result<i64> {
        let modified_timestamp = file
            .modified
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let scan_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let hash_str = hash.map(|h| h.as_str());

        let mut stmt = self.sqlite_conn.prepare(
            "INSERT OR REPLACE INTO scanned_files 
             (path, size, modified_time, file_hash, scan_timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        let file_id = stmt.insert(params![
            file.path.to_string_lossy(),
            file.size as i64,
            modified_timestamp,
            hash_str,
            scan_timestamp,
        ])?;

        Ok(file_id)
    }

    pub fn store_perceptual_hash(&self, file_id: i64, hash: &PerceptualHash) -> Result<()> {
        let hash_json = serde_json::to_string(hash)?;

        self.sqlite_conn.execute(
            "UPDATE scanned_files SET perceptual_hash = ?1 WHERE id = ?2",
            params![hash_json, file_id],
        )?;

        Ok(())
    }

    pub fn store_quality_score(&self, file_id: i64, score: &QualityScore) -> Result<()> {
        self.sqlite_conn.execute(
            "UPDATE scanned_files SET quality_score = ?1 WHERE id = ?2",
            params![score.overall, file_id],
        )?;

        Ok(())
    }

    pub fn find_exact_duplicates(&self) -> Result<Vec<DuplicateGroup>> {
        let mut stmt = self.sqlite_conn.prepare(
            "SELECT file_hash, GROUP_CONCAT(id) as file_ids 
             FROM scanned_files 
             WHERE file_hash IS NOT NULL 
             GROUP BY file_hash 
             HAVING COUNT(*) > 1",
        )?;

        let mut groups = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (_hash, file_ids_str) = row?;
            let file_ids: Vec<i64> = file_ids_str
                .split(',')
                .filter_map(|s| s.parse().ok())
                .collect();

            if file_ids.len() > 1 {
                let files = self.get_files_by_ids(&file_ids)?;
                groups.push(DuplicateGroup {
                    group_id: Uuid::new_v4(),
                    duplicate_type: DuplicateType::Exact,
                    files,
                    similarity_score: Some(1.0),
                });
            }
        }

        Ok(groups)
    }

    fn get_files_by_ids(&self, ids: &[i64]) -> Result<Vec<ImageFile>> {
        let mut files = Vec::new();

        for &id in ids {
            let mut stmt = self
                .sqlite_conn
                .prepare("SELECT path, size, modified_time FROM scanned_files WHERE id = ?1")?;

            let row = stmt.query_row(params![id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;

            let (path_str, size, modified_timestamp) = row;
            let path = PathBuf::from(path_str);
            let modified =
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(modified_timestamp as u64);

            // Detect format from file extension
            let format = Self::detect_format(&path);

            files.push(ImageFile {
                path,
                size: size as u64,
                modified,
                format,
                dimensions: None,
            });
        }

        Ok(files)
    }

    pub fn get_config(&self) -> Result<AppConfig> {
        let mut config = AppConfig::default();

        let mut stmt = self
            .sqlite_conn
            .prepare("SELECT key, value FROM app_config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "similarity_threshold" => {
                    if let Ok(val) = value.parse::<f64>() {
                        config.similarity_threshold = val;
                    }
                }
                "supported_formats" => {
                    if let Ok(formats) = serde_json::from_str::<Vec<String>>(&value) {
                        config.supported_formats = formats;
                    }
                }
                "max_scan_depth" => {
                    if let Ok(depth) = value.parse::<usize>() {
                        config.max_scan_depth = Some(depth);
                    }
                }
                "parallel_workers" => {
                    if let Ok(workers) = value.parse::<usize>() {
                        config.parallel_workers = workers;
                    }
                }
                "cache_enabled" => {
                    config.cache_enabled = value == "true";
                }
                "ai_features_enabled" => {
                    config.ai_features_enabled = value == "true";
                }
                _ => {}
            }
        }

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let configs = vec![
            (
                "similarity_threshold",
                config.similarity_threshold.to_string(),
            ),
            (
                "supported_formats",
                serde_json::to_string(&config.supported_formats)?,
            ),
            (
                "max_scan_depth",
                config
                    .max_scan_depth
                    .map_or("null".to_string(), |d| d.to_string()),
            ),
            ("parallel_workers", config.parallel_workers.to_string()),
            ("cache_enabled", config.cache_enabled.to_string()),
            (
                "ai_features_enabled",
                config.ai_features_enabled.to_string(),
            ),
        ];

        for (key, value) in configs {
            self.sqlite_conn.execute(
                "INSERT OR REPLACE INTO app_config (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params![key, value, timestamp],
            )?;
        }

        Ok(())
    }

    // Cache operations using Sled
    pub fn cache_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.sled_db.get(key)?.map(|v| v.to_vec()))
    }

    pub fn cache_set(&self, key: &str, value: &[u8]) -> Result<()> {
        self.sled_db.insert(key, value)?;
        Ok(())
    }

    pub fn cache_remove(&self, key: &str) -> Result<()> {
        self.sled_db.remove(key)?;
        Ok(())
    }

    fn detect_format(path: &std::path::Path) -> crate::services::scanner::ImageFormat {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "jpg" | "jpeg" => crate::services::scanner::ImageFormat::Jpeg,
                "png" => crate::services::scanner::ImageFormat::Png,
                "tiff" | "tif" => crate::services::scanner::ImageFormat::Tiff,
                "raw" | "cr2" | "nef" | "arw" => crate::services::scanner::ImageFormat::Raw,
                other => crate::services::scanner::ImageFormat::Other(other.to_string()),
            }
        } else {
            crate::services::scanner::ImageFormat::Other("unknown".to_string())
        }
    }
}
