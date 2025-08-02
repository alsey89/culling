use super::{DatabaseError, Repository};
use crate::database::models::{Decision, DecisionState, NewDecision, ReasonCode};
use crate::schema::decisions;
use chrono::Utc;
use diesel::prelude::*;

pub struct DecisionRepository;

impl Repository for DecisionRepository {}

impl DecisionRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        asset_id: String,
        state: DecisionState,
        reason: ReasonCode,
        notes: Option<String>,
    ) -> Result<Decision, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        let new_decision = NewDecision {
            asset_id: asset_id.clone(),
            state: String::from(state),
            reason: String::from(reason),
            notes,
            decided_at: now,
        };

        diesel::insert_into(decisions::table)
            .values(&new_decision)
            .on_conflict(decisions::asset_id)
            .do_update()
            .set((
                decisions::state.eq(&new_decision.state),
                decisions::reason.eq(&new_decision.reason),
                decisions::notes.eq(&new_decision.notes),
                decisions::decided_at.eq(&new_decision.decided_at),
            ))
            .execute(&mut conn)?;

        self.find_by_asset_id(&asset_id)
    }

    pub fn create_batch(
        &self,
        decisions_data: Vec<(String, DecisionState, ReasonCode, Option<String>)>,
    ) -> Result<Vec<Decision>, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        let new_decisions: Vec<NewDecision> = decisions_data
            .iter()
            .map(|(asset_id, state, reason, notes)| NewDecision {
                asset_id: asset_id.clone(),
                state: String::from(state.clone()),
                reason: String::from(reason.clone()),
                notes: notes.clone(),
                decided_at: now.clone(),
            })
            .collect();

        conn.transaction::<_, DatabaseError, _>(|conn| {
            for decision in &new_decisions {
                diesel::insert_into(decisions::table)
                    .values(decision)
                    .on_conflict(decisions::asset_id)
                    .do_update()
                    .set((
                        decisions::state.eq(&decision.state),
                        decisions::reason.eq(&decision.reason),
                        decisions::notes.eq(&decision.notes),
                        decisions::decided_at.eq(&decision.decided_at),
                    ))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        let asset_ids: Vec<String> = decisions_data
            .iter()
            .map(|(id, _, _, _)| id.clone())
            .collect();
        self.find_by_asset_ids(&asset_ids)
    }

    pub fn find_by_asset_id(&self, asset_id: &str) -> Result<Decision, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .filter(decisions::asset_id.eq(asset_id))
            .first(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_asset_ids(&self, asset_ids: &[String]) -> Result<Vec<Decision>, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .filter(decisions::asset_id.eq_any(asset_ids))
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_project_id(&self, project_id: &str) -> Result<Vec<Decision>, DatabaseError> {
        let mut conn = self.get_connection()?;

        // Join with assets table to filter by project_id
        decisions::table
            .inner_join(
                crate::schema::assets::table.on(decisions::asset_id.eq(crate::schema::assets::id)),
            )
            .filter(crate::schema::assets::project_id.eq(project_id))
            .select(decisions::all_columns)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_state(
        &self,
        project_id: &str,
        state: DecisionState,
    ) -> Result<Vec<Decision>, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .inner_join(
                crate::schema::assets::table.on(decisions::asset_id.eq(crate::schema::assets::id)),
            )
            .filter(crate::schema::assets::project_id.eq(project_id))
            .filter(decisions::state.eq(String::from(state)))
            .select(decisions::all_columns)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_by_reason(
        &self,
        project_id: &str,
        reason: ReasonCode,
    ) -> Result<Vec<Decision>, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .inner_join(
                crate::schema::assets::table.on(decisions::asset_id.eq(crate::schema::assets::id)),
            )
            .filter(crate::schema::assets::project_id.eq(project_id))
            .filter(decisions::reason.eq(String::from(reason)))
            .select(decisions::all_columns)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn get_decision_stats(&self, project_id: &str) -> Result<DecisionStats, DatabaseError> {
        let mut conn = self.get_connection()?;

        let decisions = self.find_by_project_id(project_id)?;

        let mut stats = DecisionStats {
            keep: 0,
            remove: 0,
            undecided: 0,
            total: decisions.len() as i64,
        };

        for decision in decisions {
            match DecisionState::from(decision.state) {
                DecisionState::Keep => stats.keep += 1,
                DecisionState::Remove => stats.remove += 1,
                DecisionState::Undecided => stats.undecided += 1,
            }
        }

        Ok(stats)
    }

    pub fn update_decision(
        &self,
        asset_id: &str,
        state: DecisionState,
        reason: ReasonCode,
        notes: Option<String>,
    ) -> Result<Decision, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        diesel::update(decisions::table.filter(decisions::asset_id.eq(asset_id)))
            .set((
                decisions::state.eq(String::from(state)),
                decisions::reason.eq(String::from(reason)),
                decisions::notes.eq(notes),
                decisions::decided_at.eq(now),
            ))
            .execute(&mut conn)?;

        self.find_by_asset_id(asset_id)
    }

    pub fn delete_by_asset_id(&self, asset_id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let deleted_count =
            diesel::delete(decisions::table.filter(decisions::asset_id.eq(asset_id)))
                .execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    pub fn delete_by_project_id(&self, project_id: &str) -> Result<usize, DatabaseError> {
        let mut conn = self.get_connection()?;

        // Delete decisions for all assets in the project
        let asset_ids: Vec<String> = crate::schema::assets::table
            .filter(crate::schema::assets::project_id.eq(project_id))
            .select(crate::schema::assets::id)
            .load(&mut conn)?;

        diesel::delete(decisions::table.filter(decisions::asset_id.eq_any(&asset_ids)))
            .execute(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn clear_all_decisions(&self, project_id: &str) -> Result<usize, DatabaseError> {
        self.delete_by_project_id(project_id)
    }

    pub fn exists(&self, asset_id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let count: i64 = decisions::table
            .filter(decisions::asset_id.eq(asset_id))
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }

    pub fn get_keep_assets(&self, project_id: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .inner_join(
                crate::schema::assets::table.on(decisions::asset_id.eq(crate::schema::assets::id)),
            )
            .filter(crate::schema::assets::project_id.eq(project_id))
            .filter(decisions::state.eq(String::from(DecisionState::Keep)))
            .select(decisions::asset_id)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn get_remove_assets(&self, project_id: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self.get_connection()?;

        decisions::table
            .inner_join(
                crate::schema::assets::table.on(decisions::asset_id.eq(crate::schema::assets::id)),
            )
            .filter(crate::schema::assets::project_id.eq(project_id))
            .filter(decisions::state.eq(String::from(DecisionState::Remove)))
            .select(decisions::asset_id)
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }
}

#[derive(Debug, Clone)]
pub struct DecisionStats {
    pub keep: i64,
    pub remove: i64,
    pub undecided: i64,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::init_database;
    use crate::database::repositories::{AssetRepository, ProjectRepository};
    use std::env;
    use tempfile::tempdir;

    fn setup_test_db() -> (String, String) {
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

        // Create a test asset
        let asset_repo = AssetRepository::new();
        let asset = asset_repo
            .create(
                project.id.clone(),
                "/test/image.jpg".to_string(),
                Some("hash123".to_string()),
                None,
                1024000,
                1920,
                1080,
                None,
            )
            .unwrap();

        (project.id, asset.id)
    }

    #[test]
    fn test_create_decision() {
        let (project_id, asset_id) = setup_test_db();
        let repo = DecisionRepository::new();

        let decision = repo
            .create(
                asset_id.clone(),
                DecisionState::Keep,
                ReasonCode::UserOverrideKeep,
                Some("User selected this image".to_string()),
            )
            .unwrap();

        assert_eq!(decision.asset_id, asset_id);
        assert_eq!(decision.state, String::from(DecisionState::Keep));
        assert_eq!(decision.reason, String::from(ReasonCode::UserOverrideKeep));
        assert_eq!(decision.notes, Some("User selected this image".to_string()));
    }

    #[test]
    fn test_update_decision() {
        let (project_id, asset_id) = setup_test_db();
        let repo = DecisionRepository::new();

        // Create initial decision
        repo.create(
            asset_id.clone(),
            DecisionState::Undecided,
            ReasonCode::ManualNoReason,
            None,
        )
        .unwrap();

        // Update the decision
        let updated = repo
            .update_decision(
                &asset_id,
                DecisionState::Remove,
                ReasonCode::UserOverrideRemove,
                Some("Changed mind".to_string()),
            )
            .unwrap();

        assert_eq!(updated.state, String::from(DecisionState::Remove));
        assert_eq!(updated.reason, String::from(ReasonCode::UserOverrideRemove));
        assert_eq!(updated.notes, Some("Changed mind".to_string()));
    }

    #[test]
    fn test_decision_stats() {
        let (project_id, _) = setup_test_db();
        let repo = DecisionRepository::new();
        let asset_repo = AssetRepository::new();

        // Create multiple assets with different decisions
        for i in 1..=5 {
            let asset = asset_repo
                .create(
                    project_id.clone(),
                    format!("/test/image{}.jpg", i),
                    Some(format!("hash{}", i)),
                    None,
                    1024000,
                    1920,
                    1080,
                    None,
                )
                .unwrap();

            let state = match i {
                1..=2 => DecisionState::Keep,
                3..=4 => DecisionState::Remove,
                _ => DecisionState::Undecided,
            };

            repo.create(asset.id, state, ReasonCode::ManualNoReason, None)
                .unwrap();
        }

        let stats = repo.get_decision_stats(&project_id).unwrap();
        assert_eq!(stats.keep, 2);
        assert_eq!(stats.remove, 2);
        assert_eq!(stats.undecided, 1);
        assert_eq!(stats.total, 5);
    }

    #[test]
    fn test_find_by_state() {
        let (project_id, _) = setup_test_db();
        let repo = DecisionRepository::new();
        let asset_repo = AssetRepository::new();

        // Create assets with keep decisions
        for i in 1..=3 {
            let asset = asset_repo
                .create(
                    project_id.clone(),
                    format!("/test/image{}.jpg", i),
                    Some(format!("hash{}", i)),
                    None,
                    1024000,
                    1920,
                    1080,
                    None,
                )
                .unwrap();

            repo.create(
                asset.id,
                DecisionState::Keep,
                ReasonCode::UserOverrideKeep,
                None,
            )
            .unwrap();
        }

        let keep_decisions = repo
            .find_by_state(&project_id, DecisionState::Keep)
            .unwrap();
        assert_eq!(keep_decisions.len(), 3);
    }

    #[test]
    fn test_batch_create() {
        let (project_id, _) = setup_test_db();
        let repo = DecisionRepository::new();
        let asset_repo = AssetRepository::new();

        // Create multiple assets
        let mut asset_ids = Vec::new();
        for i in 1..=3 {
            let asset = asset_repo
                .create(
                    project_id.clone(),
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

        // Create batch decisions
        let decisions_data = asset_ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    DecisionState::Keep,
                    ReasonCode::UserOverrideKeep,
                    None,
                )
            })
            .collect();

        let decisions = repo.create_batch(decisions_data).unwrap();
        assert_eq!(decisions.len(), 3);

        for decision in decisions {
            assert_eq!(decision.state, String::from(DecisionState::Keep));
        }
    }
}
