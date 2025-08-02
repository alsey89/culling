pub mod asset;
pub mod decision;
pub mod project;
pub mod variant_group;

pub use asset::AssetRepository;
pub use decision::DecisionRepository;
pub use project::ProjectRepository;
pub use variant_group::VariantGroupRepository;

use super::DatabaseError;
use crate::database::connection::get_connection;
use crate::database::DbConnection;

pub trait Repository {
    fn get_connection(&self) -> Result<DbConnection, DatabaseError> {
        get_connection()
    }
}
