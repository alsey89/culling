use super::{establish_connection, get_database_path, DatabaseError, DbPool};
use std::sync::OnceLock;

static DB_POOL: OnceLock<DbPool> = OnceLock::new();

pub fn init_database() -> Result<(), DatabaseError> {
    let database_url = get_database_path()?;
    let pool = establish_connection(&database_url)?;

    DB_POOL
        .set(pool)
        .map_err(|_| DatabaseError::Migration("Failed to initialize database pool".to_string()))?;

    Ok(())
}

pub fn get_connection() -> Result<super::DbConnection, DatabaseError> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| DatabaseError::Migration("Database not initialized".to_string()))?;

    pool.get()
        .map_err(|e| super::DatabaseError::Migration(format!("Pool connection failed: {}", e)))
}
