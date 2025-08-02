use super::{DatabaseError, Repository};
use crate::database::models::{AssetGroup, GroupType, NewVariantGroup, VariantGroup};
use crate::schema::{asset_groups, variant_groups};
use chrono::Utc;
use diesel::prelude::*;
use uuid::Uuid;

pub struct VariantGroupRepository;

impl Repository for VariantGroupRepository {}

impl VariantGroupRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        project_id: String,
        group_type: GroupType,
        similarity: f32,
        suggested_keep: Option<String>,
        asset_ids: Vec<String>,
    ) -> Result<VariantGroup, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();
        let id = format!("grp_{}", Uuid::new_v4().simple());

        let new_group = NewVariantGroup {
            id: id.clone(),
            project_id,
            group_type: String::from(group_type),
            similarity,
            suggested_keep,
            created_at: now,
        };

        conn.transaction::<_, DatabaseError, _>(|conn| {
            // Insert the variant group
            diesel::insert_into(variant_groups::table)
                .values(&new_group)
                .execute(conn)?;

            // Insert asset group memberships
            let asset_group_memberships: Vec<AssetGroup> = asset_ids
                .iter()
                .map(|asset_id| AssetGroup {
                    asset_id: asset_id.clone(),
                    group_id: id.clone(),
                })
                .collect();

            diesel::insert_into(asset_groups::table)
                .values(&asset_group_memberships)
                .execute(conn)?;

            Ok(())
        })?;

        self.find_by_id(&id)
    }

    pub fn find_by_id(&self, id: &str) -> Result<VariantGroup, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .filter(variant_groups::id.eq(id))
            .first(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_project_id(&self, project_id: &str) -> Result<Vec<VariantGroup>, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .filter(variant_groups::project_id.eq(project_id))
            .order(variant_groups::created_at.desc())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_type(
        &self,
        project_id: &str,
        group_type: GroupType,
    ) -> Result<Vec<VariantGroup>, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .filter(variant_groups::project_id.eq(project_id))
            .filter(variant_groups::group_type.eq(String::from(group_type)))
            .order(variant_groups::created_at.desc())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn get_asset_ids_for_group(&self, group_id: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self.get_connection()?;

        asset_groups::table
            .filter(asset_groups::group_id.eq(group_id))
            .select(asset_groups::asset_id)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn get_groups_for_asset(&self, asset_id: &str) -> Result<Vec<VariantGroup>, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .inner_join(asset_groups::table.on(variant_groups::id.eq(asset_groups::group_id)))
            .filter(asset_groups::asset_id.eq(asset_id))
            .select(variant_groups::all_columns)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn update_suggested_keep(
        &self,
        group_id: &str,
        suggested_keep: Option<String>,
    ) -> Result<VariantGroup, DatabaseError> {
        let mut conn = self.get_connection()?;

        diesel::update(variant_groups::table.filter(variant_groups::id.eq(group_id)))
            .set(variant_groups::suggested_keep.eq(suggested_keep))
            .execute(&mut conn)?;

        self.find_by_id(group_id)
    }

    pub fn add_asset_to_group(&self, group_id: &str, asset_id: &str) -> Result<(), DatabaseError> {
        let mut conn = self.get_connection()?;

        let asset_group = AssetGroup {
            asset_id: asset_id.to_string(),
            group_id: group_id.to_string(),
        };

        diesel::insert_into(asset_groups::table)
            .values(&asset_group)
            .on_conflict((asset_groups::asset_id, asset_groups::group_id))
            .do_nothing()
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn remove_asset_from_group(
        &self,
        group_id: &str,
        asset_id: &str,
    ) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let deleted_count = diesel::delete(
            asset_groups::table
                .filter(asset_groups::group_id.eq(group_id))
                .filter(asset_groups::asset_id.eq(asset_id)),
        )
        .execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    pub fn delete_group(&self, group_id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        conn.transaction::<_, DatabaseError, _>(|conn| {
            // Delete asset group memberships first
            diesel::delete(asset_groups::table.filter(asset_groups::group_id.eq(group_id)))
                .execute(conn)?;

            // Delete the variant group
            let deleted_count =
                diesel::delete(variant_groups::table.filter(variant_groups::id.eq(group_id)))
                    .execute(conn)?;

            Ok(deleted_count > 0)
        })
    }

    pub fn delete_by_project_id(&self, project_id: &str) -> Result<usize, DatabaseError> {
        let mut conn = self.get_connection()?;

        conn.transaction::<_, DatabaseError, _>(|conn| {
            // Get all group IDs for this project
            let group_ids: Vec<String> = variant_groups::table
                .filter(variant_groups::project_id.eq(project_id))
                .select(variant_groups::id)
                .load(conn)?;

            // Delete asset group memberships
            diesel::delete(asset_groups::table.filter(asset_groups::group_id.eq_any(&group_ids)))
                .execute(conn)?;

            // Delete variant groups
            let deleted_count = diesel::delete(
                variant_groups::table.filter(variant_groups::project_id.eq(project_id)),
            )
            .execute(conn)?;

            Ok(deleted_count)
        })
    }

    pub fn count_by_project_id(&self, project_id: &str) -> Result<i64, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .filter(variant_groups::project_id.eq(project_id))
            .count()
            .get_result(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn count_by_type(
        &self,
        project_id: &str,
        group_type: GroupType,
    ) -> Result<i64, DatabaseError> {
        let mut conn = self.get_connection()?;

        variant_groups::table
            .filter(variant_groups::project_id.eq(project_id))
            .filter(variant_groups::group_type.eq(String::from(group_type)))
            .count()
            .get_result(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn get_group_stats(&self, project_id: &str) -> Result<GroupStats, DatabaseError> {
        let mut conn = self.get_connection()?;

        let exact_count = self.count_by_type(project_id, GroupType::Exact)?;
        let similar_count = self.count_by_type(project_id, GroupType::Similar)?;
        let total_count = exact_count + similar_count;

        // Calculate total assets in groups
        let total_assets_in_groups: i64 = asset_groups::table
            .inner_join(variant_groups::table.on(asset_groups::group_id.eq(variant_groups::id)))
            .filter(variant_groups::project_id.eq(project_id))
            .count()
            .get_result(&mut conn)?;

        Ok(GroupStats {
            total_groups: total_count,
            exact_groups: exact_count,
            similar_groups: similar_count,
            total_assets_in_groups,
        })
    }

    pub fn exists(&self, group_id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let count: i64 = variant_groups::table
            .filter(variant_groups::id.eq(group_id))
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }

    pub fn get_empty_groups(&self, project_id: &str) -> Result<Vec<VariantGroup>, DatabaseError> {
        let mut conn = self.get_connection()?;

        // Find groups that have no assets
        variant_groups::table
            .left_join(asset_groups::table.on(variant_groups::id.eq(asset_groups::group_id)))
            .filter(variant_groups::project_id.eq(project_id))
            .filter(asset_groups::group_id.is_null())
            .select(variant_groups::all_columns)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn cleanup_empty_groups(&self, project_id: &str) -> Result<usize, DatabaseError> {
        let mut conn = self.get_connection()?;

        // First, find empty group IDs
        let empty_group_ids: Vec<String> = variant_groups::table
            .left_join(asset_groups::table.on(variant_groups::id.eq(asset_groups::group_id)))
            .filter(variant_groups::project_id.eq(project_id))
            .filter(asset_groups::group_id.is_null())
            .select(variant_groups::id)
            .load(&mut conn)?;

        // Delete the empty groups
        let deleted_count = diesel::delete(
            variant_groups::table.filter(variant_groups::id.eq_any(&empty_group_ids)),
        )
        .execute(&mut conn)?;

        Ok(deleted_count)
    }
}

#[derive(Debug, Clone)]
pub struct GroupStats {
    pub total_groups: i64,
    pub exact_groups: i64,
    pub similar_groups: i64,
    pub total_assets_in_groups: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::init_database;
    use crate::database::repositories::{AssetRepository, ProjectRepository};
    use std::env;
    use tempfile::tempdir;

    fn setup_test_db() -> (String, Vec<String>) {
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

        // Create test assets
        let asset_repo = AssetRepository::new();
        let mut asset_ids = Vec::new();
        for i in 1..=3 {
            let asset = asset_repo
                .create(
                    project.id.clone(),
                    format!("/test/image{}.jpg", i),
                    Some(format!("hash{}", i)),
                    None,
                    1024000,
                    1920,
                    1080,
                    None,
                )
                .unwrap();
            asset_ids.push(asset.id);
        }

        (project.id, asset_ids)
    }

    #[test]
    fn test_create_variant_group() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        let group = repo
            .create(
                project_id.clone(),
                GroupType::Exact,
                100.0,
                Some(asset_ids[0].clone()),
                asset_ids.clone(),
            )
            .unwrap();

        assert_eq!(group.project_id, project_id);
        assert_eq!(group.group_type, String::from(GroupType::Exact));
        assert_eq!(group.similarity, 100.0);
        assert_eq!(group.suggested_keep, Some(asset_ids[0].clone()));
        assert!(group.id.starts_with("grp_"));
    }

    #[test]
    fn test_get_asset_ids_for_group() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        let group = repo
            .create(project_id, GroupType::Exact, 100.0, None, asset_ids.clone())
            .unwrap();

        let group_asset_ids = repo.get_asset_ids_for_group(&group.id).unwrap();
        assert_eq!(group_asset_ids.len(), 3);

        // Check that all original asset IDs are present
        for asset_id in &asset_ids {
            assert!(group_asset_ids.contains(asset_id));
        }
    }

    #[test]
    fn test_find_by_type() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        // Create exact duplicate group
        repo.create(
            project_id.clone(),
            GroupType::Exact,
            100.0,
            None,
            vec![asset_ids[0].clone(), asset_ids[1].clone()],
        )
        .unwrap();

        // Create similar group
        repo.create(
            project_id.clone(),
            GroupType::Similar,
            85.0,
            None,
            vec![asset_ids[1].clone(), asset_ids[2].clone()],
        )
        .unwrap();

        let exact_groups = repo.find_by_type(&project_id, GroupType::Exact).unwrap();
        let similar_groups = repo.find_by_type(&project_id, GroupType::Similar).unwrap();

        assert_eq!(exact_groups.len(), 1);
        assert_eq!(similar_groups.len(), 1);
        assert_eq!(exact_groups[0].similarity, 100.0);
        assert_eq!(similar_groups[0].similarity, 85.0);
    }

    #[test]
    fn test_update_suggested_keep() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        let group = repo
            .create(project_id, GroupType::Exact, 100.0, None, asset_ids.clone())
            .unwrap();

        let updated = repo
            .update_suggested_keep(&group.id, Some(asset_ids[1].clone()))
            .unwrap();
        assert_eq!(updated.suggested_keep, Some(asset_ids[1].clone()));
    }

    #[test]
    fn test_add_remove_asset() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        // Create group with first two assets
        let group = repo
            .create(
                project_id.clone(),
                GroupType::Similar,
                90.0,
                None,
                vec![asset_ids[0].clone(), asset_ids[1].clone()],
            )
            .unwrap();

        // Add third asset
        repo.add_asset_to_group(&group.id, &asset_ids[2]).unwrap();
        let group_assets = repo.get_asset_ids_for_group(&group.id).unwrap();
        assert_eq!(group_assets.len(), 3);

        // Remove first asset
        let removed = repo
            .remove_asset_from_group(&group.id, &asset_ids[0])
            .unwrap();
        assert!(removed);

        let group_assets = repo.get_asset_ids_for_group(&group.id).unwrap();
        assert_eq!(group_assets.len(), 2);
        assert!(!group_assets.contains(&asset_ids[0]));
    }

    #[test]
    fn test_group_stats() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        // Create exact duplicate group
        repo.create(
            project_id.clone(),
            GroupType::Exact,
            100.0,
            None,
            vec![asset_ids[0].clone(), asset_ids[1].clone()],
        )
        .unwrap();

        // Create similar group
        repo.create(
            project_id.clone(),
            GroupType::Similar,
            85.0,
            None,
            vec![asset_ids[2].clone()],
        )
        .unwrap();

        let stats = repo.get_group_stats(&project_id).unwrap();
        assert_eq!(stats.total_groups, 2);
        assert_eq!(stats.exact_groups, 1);
        assert_eq!(stats.similar_groups, 1);
        assert_eq!(stats.total_assets_in_groups, 3);
    }

    #[test]
    fn test_delete_group() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        let group = repo
            .create(project_id, GroupType::Exact, 100.0, None, asset_ids)
            .unwrap();

        let deleted = repo.delete_group(&group.id).unwrap();
        assert!(deleted);

        let exists = repo.exists(&group.id).unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_cleanup_empty_groups() {
        let (project_id, asset_ids) = setup_test_db();
        let repo = VariantGroupRepository::new();

        // Create a group
        let group = repo
            .create(
                project_id.clone(),
                GroupType::Exact,
                100.0,
                None,
                asset_ids.clone(),
            )
            .unwrap();

        // Remove all assets from the group
        for asset_id in &asset_ids {
            repo.remove_asset_from_group(&group.id, asset_id).unwrap();
        }

        // Check that group is now empty
        let empty_groups = repo.get_empty_groups(&project_id).unwrap();
        assert_eq!(empty_groups.len(), 1);
        assert_eq!(empty_groups[0].id, group.id);

        // Cleanup empty groups
        let cleaned_count = repo.cleanup_empty_groups(&project_id).unwrap();
        assert_eq!(cleaned_count, 1);

        let exists = repo.exists(&group.id).unwrap();
        assert!(!exists);
    }
}
