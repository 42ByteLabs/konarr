use geekorm::ConnectionManager;

use super::DatabaseConfig;
use crate::KonarrError as Error;

impl DatabaseConfig {
    /// Create / Connect to the Database
    ///
    /// Only support SQLite for now using LibSQL
    ///
    /// Supported formats:
    ///
    /// - `:memory:` - In-Memory SQLite Database, not persisted
    /// - `./path/to/database.db` - Relative path to SQLite database
    /// - `/path/to/database.db` - Absolute path to SQLite database
    /// - `libsql:libsql.42bytelabs.com` - Remote LibSQL Database
    ///
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database() -> Result<(), Error> {
        let config = DatabaseConfig {
            path: Some(":memory:".to_string()),
            ..Default::default()
        };

        let _conn = config.connection().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_connection() -> Result<(), Error> {
        let config = DatabaseConfig {
            path: Some("/tmp/konarr-test.db".to_string()),
            ..Default::default()
        };

        let _conn = config.connection().await?;

        assert_eq!(std::fs::metadata("/tmp/konarr-test.db")?.is_file(), true);

        Ok(())
    }
}
