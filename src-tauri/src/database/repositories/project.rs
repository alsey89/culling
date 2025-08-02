use super::{DatabaseError, Repository};
use crate::database::models::{NewProject, Project, ScanStatus};
use crate::schema::projects;
use chrono::Utc;
use diesel::prelude::*;
use serde_json;
use uuid::Uuid;

pub struct ProjectRepository;

impl Repository for ProjectRepository {}

impl ProjectRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn create(
        &self,
        name: String,
        source_path: String,
        output_path: String,
        exclude_patterns: Vec<String>,
        file_types: Vec<String>,
    ) -> Result<Project, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();
        let id = format!("prj_{}", Uuid::new_v4().simple());

        let new_project = NewProject {
            id: id.clone(),
            name: name.clone(),
            source_path: source_path.clone(),
            output_path: output_path.clone(),
            exclude_patterns: serde_json::to_string(&exclude_patterns)?,
            file_types: serde_json::to_string(&file_types)?,
            scan_status: String::from(ScanStatus::NotStarted),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        diesel::insert_into(projects::table)
            .values(&new_project)
            .execute(&mut conn)?;

        self.find_by_id(&id)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Project, DatabaseError> {
        let mut conn = self.get_connection()?;

        projects::table
            .filter(projects::id.eq(id))
            .first(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn find_all(&self) -> Result<Vec<Project>, DatabaseError> {
        let mut conn = self.get_connection()?;

        projects::table
            .order(projects::created_at.desc())
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }

    pub fn update_scan_status(
        &self,
        id: &str,
        status: ScanStatus,
    ) -> Result<Project, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        diesel::update(projects::table.filter(projects::id.eq(id)))
            .set((
                projects::scan_status.eq(String::from(status)),
                projects::updated_at.eq(now),
            ))
            .execute(&mut conn)?;

        self.find_by_id(id)
    }

    pub fn update(
        &self,
        id: &str,
        name: Option<String>,
        source_path: Option<String>,
        output_path: Option<String>,
        exclude_patterns: Option<Vec<String>>,
        file_types: Option<Vec<String>>,
    ) -> Result<Project, DatabaseError> {
        let mut conn = self.get_connection()?;
        let now = Utc::now().to_rfc3339();

        let mut changeset = Vec::new();

        if let Some(name) = name {
            changeset.push(("name", name));
        }
        if let Some(source_path) = source_path {
            changeset.push(("source_path", source_path));
        }
        if let Some(output_path) = output_path {
            changeset.push(("output_path", output_path));
        }
        if let Some(exclude_patterns) = exclude_patterns {
            changeset.push((
                "exclude_patterns",
                serde_json::to_string(&exclude_patterns)?,
            ));
        }
        if let Some(file_types) = file_types {
            changeset.push(("file_types", serde_json::to_string(&file_types)?));
        }

        if !changeset.is_empty() {
            diesel::update(projects::table.filter(projects::id.eq(id)))
                .set(projects::updated_at.eq(now))
                .execute(&mut conn)?;
        }

        self.find_by_id(id)
    }

    pub fn delete(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let deleted_count =
            diesel::delete(projects::table.filter(projects::id.eq(id))).execute(&mut conn)?;

        Ok(deleted_count > 0)
    }

    pub fn exists(&self, id: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.get_connection()?;

        let count: i64 = projects::table
            .filter(projects::id.eq(id))
            .count()
            .get_result(&mut conn)?;

        Ok(count > 0)
    }

    pub fn find_by_source_path(&self, source_path: &str) -> Result<Vec<Project>, DatabaseError> {
        let mut conn = self.get_connection()?;

        projects::table
            .filter(projects::source_path.eq(source_path))
            .load(&mut conn)
            .map_err(DatabaseError::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::connection::init_database;
    use std::env;
    use tempfile::tempdir;

    fn setup_test_db() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let temp_dir = tempdir().unwrap();
            let db_path = temp_dir.path().join("test.db");
            env::set_var("DATABASE_URL", format!("sqlite://{}", db_path.display()));
            init_database().unwrap();
        });
    }

    #[test]
    fn test_create_project() {
        setup_test_db();
        let repo = ProjectRepository::new();

        let project = repo
            .create(
                "Test Project".to_string(),
                "/test/source".to_string(),
                "/test/output".to_string(),
                vec!["*.tmp".to_string()],
                vec!["jpg".to_string(), "png".to_string()],
            )
            .unwrap();

        assert_eq!(project.name, "Test Project");
        assert_eq!(project.source_path, "/test/source");
        assert_eq!(project.output_path, "/test/output");
        assert!(project.id.starts_with("prj_"));
    }

    #[test]
    fn test_find_by_id() {
        setup_test_db();
        let repo = ProjectRepository::new();

        let created = repo
            .create(
                "Test Project".to_string(),
                "/test/source".to_string(),
                "/test/output".to_string(),
                vec![],
                vec!["jpg".to_string()],
            )
            .unwrap();

        let found = repo.find_by_id(&created.id).unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.name, created.name);
    }

    #[test]
    fn test_update_scan_status() {
        setup_test_db();
        let repo = ProjectRepository::new();

        let project = repo
            .create(
                "Test Project".to_string(),
                "/test/source".to_string(),
                "/test/output".to_string(),
                vec![],
                vec!["jpg".to_string()],
            )
            .unwrap();

        let updated = repo
            .update_scan_status(&project.id, ScanStatus::InProgress)
            .unwrap();
        assert_eq!(updated.scan_status, String::from(ScanStatus::InProgress));
    }

    #[test]
    fn test_delete_project() {
        setup_test_db();
        let repo = ProjectRepository::new();

        let project = repo
            .create(
                "Test Project".to_string(),
                "/test/source".to_string(),
                "/test/output".to_string(),
                vec![],
                vec!["jpg".to_string()],
            )
            .unwrap();

        let deleted = repo.delete(&project.id).unwrap();
        assert!(deleted);

        let exists = repo.exists(&project.id).unwrap();
        assert!(!exists);
    }
}
