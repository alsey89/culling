pub mod connection;
pub mod models;
pub mod repositories;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use thiserror::Error;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] diesel::ConnectionError),

    #[error("Database query error: {0}")]
    Query(#[from] diesel::result::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] diesel::r2d2::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn establish_connection(database_url: &str) -> Result<DbPool, DatabaseError> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(10)
        .build(manager)
        .map_err(|e| DatabaseError::Migration(format!("Pool creation failed: {}", e)))?;

    // Run migrations
    let mut conn = pool
        .get()
        .map_err(|e| DatabaseError::Migration(format!("Pool connection failed: {}", e)))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DatabaseError::Migration(e.to_string()))?;

    Ok(pool)
}

pub fn get_database_path() -> Result<String, DatabaseError> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| DatabaseError::Migration("Could not find home directory".to_string()))?;

    let app_dir = home_dir.join("Documents").join("Cullrs");
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| DatabaseError::Migration(format!("Could not create app directory: {}", e)))?;

    let db_path = app_dir.join("cullrs.db");
    Ok(db_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_connection() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());

        let pool = establish_connection(&database_url).unwrap();
        let mut conn = pool.get().unwrap();

        // Test that we can execute a simple query
        use diesel::sql_types::Integer;

        #[derive(QueryableByName)]
        struct TestResult {
            #[diesel(sql_type = Integer)]
            test: i32,
        }

        let result: TestResult = diesel::sql_query("SELECT 1 as test")
            .get_result(&mut conn)
            .unwrap();

        assert_eq!(result.test, 1);
    }
}
