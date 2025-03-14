use geekorm::ConnectionManager;

use super::DatabaseConfig;
use crate::KonarrError as Error;

impl DatabaseConfig {
    /// Create / Connect to the Database
    pub async fn database(&self) -> Result<ConnectionManager, Error> {
        if let Some(path) = &self.path {
            Ok(ConnectionManager::connect(path).await?)
        } else {
            log::info!("Connecting to In-Memory Database");
            log::warn!("In-Memory Database is not persisted and will be lost on restart");

            Ok(ConnectionManager::in_memory().await?)
        }
    }
}
