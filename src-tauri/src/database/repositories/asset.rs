use super::{DatabaseError, Repository};
use crate::database::models::{Asset, ExifData, NewAsset};
use crate::schema::assets;
use chrono::Utc;
use diesel::prelude::*;
use serde_json;
use uuid::Uuid;

pub struct AssetRepository;

impl Repository for AssetRepository {}

impl AssetRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        project_id: String,
        path: String,
        thumbnail_path: Option<String>,
        hash: Option<String>,
        perceptual_hash: Option<String>,
        size: i32,
        width: i32,
        height: i32,
        exif_data: Option<ExifData>,
    ) -> Result<Asset, DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let id = format!("ast_{}", Uuid::new_v4().simple());

        let exif_json = match exif_data {
            Some(data) => Some(serde_json::to_string(&data)?),
            None => None,
        };

        let new_asset = NewAsset {
            id: id.clone(),
            project_id,
            path,
            thumbnail_path,
            hash,
            perceptual_hash,
            size,
            width,
            height,
            exif_data: exif_json,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let mut conn = self.get_connection()?;
        diesel::insert_into(assets::table)
            .values(&new_asset)
            .execute(&mut conn)?;

        self.find_by_id(&id)
    }

    pub fn create_batch(&self, assets_data: Vec<NewAsset>) -> Result<Vec<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::insert_into(assets::table)
            .values(&assets_data)
            .execute(&mut conn)?;

        // Return the created assets by their IDs
        let ids: Vec<String> = assets_data.iter().map(|a| a.id.clone()).collect();
        self.find_by_ids(&ids)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Asset, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::id.eq(id))
            .select(Asset::as_select())
            .first(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_ids(&self, ids: &[String]) -> Result<Vec<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::id.eq_any(ids))
            .select(Asset::as_select())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_project_id(&self, project_id: &str) -> Result<Vec<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::project_id.eq(project_id))
            .select(Asset::as_select())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_project_id_paginated(
        &self,
        project_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::project_id.eq(project_id))
            .order(assets::created_at.asc())
            .limit(limit)
            .offset(offset)
            .select(Asset::as_select())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_hash(&self, hash: &str) -> Result<Vec<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::hash.eq(hash))
            .select(Asset::as_select())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_duplicates_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<Vec<Asset>>, DatabaseError> {
        let mut conn = self.get_connection()?;

        // Find all assets with duplicate hashes in this project
        let duplicate_hashes: Vec<String> = assets::table
            .filter(assets::project_id.eq(project_id))
            .filter(assets::hash.is_not_null())
            .group_by(assets::hash)
            .having(diesel::dsl::count(assets::id).gt(1))
            .select(assets::hash.assume_not_null())
            .load(&mut conn)?;

        let mut duplicate_groups = Vec::new();
        for hash in duplicate_hashes {
            let duplicates = self.find_by_hash(&hash)?;
            if duplicates.len() > 1 {
                duplicate_groups.push(duplicates);
            }
        }

        Ok(duplicate_groups)
    }

    pub fn update_hash(&self, id: &str, hash: String) -> Result<Asset, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        diesel::update(assets::table.filter(assets::id.eq(id)))
            .set((assets::hash.eq(hash), assets::updated_at.eq(now)))
            .execute(&mut conn)?;

        self.find_by_id(id)
    }

    pub fn update_perceptual_hash(
        &self,
        id: &str,
        perceptual_hash: String,
    ) -> Result<Asset, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        diesel::update(assets::table.filter(assets::id.eq(id)))
            .set((
                assets::perceptual_hash.eq(perceptual_hash),
                assets::updated_at.eq(now),
            ))
            .execute(&mut conn)?;

        self.find_by_id(id)
    }

    pub fn update_batch_hashes(&self, updates: Vec<(String, String)>) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        conn.transaction::<_, DatabaseError, _>(|conn| {
            for (id, hash) in updates {
                diesel::update(assets::table.filter(assets::id.eq(&id)))
                    .set((assets::hash.eq(&hash), assets::updated_at.eq(&now)))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn update_batch_perceptual_hashes(
        &self,
        updates: Vec<(String, String)>,
    ) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        conn.transaction::<_, DatabaseError, _>(|conn| {
            for (id, perceptual_hash) in updates {
                diesel::update(assets::table.filter(assets::id.eq(&id)))
                    .set((
                        assets::perceptual_hash.eq(&perceptual_hash),
                        assets::updated_at.eq(&now),
                    ))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn count_by_project_id(&self, project_id: &str) -> Result<i64, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::project_id.eq(project_id))
            .count()
            .get_result(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn delete_by_project_id(&self, project_id: &str) -> Result<usize, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::delete(assets::table.filter(assets::project_id.eq(project_id)))
            .execute(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn delete(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let deleted_count =
            diesel::delete(assets::table.filter(assets::id.eq(id))).execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    pub fn exists(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let count: i64 = assets::table
            .filter(assets::id.eq(id))
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }

    pub fn find_by_path(&self, path: &str) -> Result<Option<Asset>, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::path.eq(path))
            .select(Asset::as_select())
            .first(&mut conn)
            .optional()
            .map_err(DatabaseError::Query)
    }

    pub fn get_total_size_by_project(&self, project_id: &str) -> Result<i64, DatabaseError> {
        let mut conn = self.get_connection()?;

        assets::table
            .filter(assets::project_id.eq(project_id))
            .select(diesel::dsl::sum(assets::size))
            .first(&mut conn)
            .map(|size: Option<i64>| size.unwrap_or(0))
            .map_err(DatabaseError::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::init_database;
    use crate::database::repositories::ProjectRepository;
    use std::env;
    use tempfile::tempdir;

    fn setup_test_db() -> String {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let temp_dir = tempdir().unwrap();
            let db_path = temp_dir.path().join("test.db");
            env::set_var("DATABASE_URL", format!("sqlite://{}", db_path.display()));
            init_database().unwrap();
        });

        // Create a test project
        let project_repo = ProjectRepository::new();
        let project = project_repo
            .create(
                "Test Project".to_string(),
                "/test/source".to_string(),
                "/test/output".to_string(),
                vec![],
                vec!["jpg".to_string()],
            )
            .unwrap();

        project.id
    }

    #[test]
    fn test_create_asset() {
        let project_id = setup_test_db();
        let repo = AssetRepository::new();

        let asset = repo
            .create(
                project_id.clone(),
                "/test/image.jpg".to_string(),
                None, // thumbnail_path
                Some("hash123".to_string()),
                Some("phash456".to_string()),
                1024000,
                1920,
                1080,
                None,
            )
            .unwrap();

        assert_eq!(asset.project_id, project_id);
        assert_eq!(asset.path, "/test/image.jpg");
        assert_eq!(asset.hash, Some("hash123".to_string()));
        assert_eq!(asset.size, 1024000);
        assert!(asset.id.starts_with("ast_"));
    }

    #[test]
    fn test_find_by_project_id() {
        let project_id = setup_test_db();
        let repo = AssetRepository::new();

        // Create multiple assets
        repo.create(
            project_id.clone(),
            "/test/image1.jpg".to_string(),
            None, // thumbnail_path
            Some("hash1".to_string()),
            None,
            1024000,
            1920,
            1080,
            None,
        )
        .unwrap();

        repo.create(
            project_id.clone(),
            "/test/image2.jpg".to_string(),
            None, // thumbnail_path
            Some("hash2".to_string()),
            None,
            2048000,
            3840,
            2160,
            None,
        )
        .unwrap();

        let assets = repo.find_by_project_id(&project_id).unwrap();
        assert_eq!(assets.len(), 2);
    }

    #[test]
    fn test_find_duplicates() {
        let project_id = setup_test_db();
        let repo = AssetRepository::new();

        // Create assets with duplicate hashes
        repo.create(
            project_id.clone(),
            "/test/image1.jpg".to_string(),
            None, // thumbnail_path
            Some("duplicate_hash".to_string()),
            None,
            1024000,
            1920,
            1080,
            None,
        )
        .unwrap();

        repo.create(
            project_id.clone(),
            "/test/image2.jpg".to_string(),
            None, // thumbnail_path
            Some("duplicate_hash".to_string()),
            None,
            1024000,
            1920,
            1080,
            None,
        )
        .unwrap();

        repo.create(
            project_id.clone(),
            "/test/image3.jpg".to_string(),
            None, // thumbnail_path
            Some("unique_hash".to_string()),
            None,
            2048000,
            3840,
            2160,
            None,
        )
        .unwrap();

        let duplicate_groups = repo.find_duplicates_by_project(&project_id).unwrap();
        assert_eq!(duplicate_groups.len(), 1);
        assert_eq!(duplicate_groups[0].len(), 2);
    }

    #[test]
    fn test_update_hash() {
        let project_id = setup_test_db();
        let repo = AssetRepository::new();

        let asset = repo
            .create(
                project_id,
                "/test/image.jpg".to_string(),
                None, // thumbnail_path
                None,
                None,
                1024000,
                1920,
                1080,
                None,
            )
            .unwrap();

        let updated = repo.update_hash(&asset.id, "new_hash".to_string()).unwrap();
        assert_eq!(updated.hash, Some("new_hash".to_string()));
    }

    #[test]
    fn test_count_by_project() {
        let project_id = setup_test_db();
        let repo = AssetRepository::new();

        // Create multiple assets
        for i in 1..=5 {
            repo.create(
                project_id.clone(),
                format!("/test/image{}.jpg", i),
                None, // thumbnail_path
                Some(format!("hash{}", i)),
                None,
                1024000,
                1920,
                1080,
                None,
            )
            .unwrap();
        }

        let count = repo.count_by_project_id(&project_id).unwrap();
        assert_eq!(count, 5);
    }
}
