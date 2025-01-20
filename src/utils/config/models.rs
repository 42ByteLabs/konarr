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
    pub async fn database(&self) -> Result<libsql::Database, Error> {
        if let Some(path) = &self.path {
            match path.as_str() {
                ":memory:" => {
                    log::info!("Connecting to In-Memory Database");
                    log::warn!("In-Memory Database is not persisted and will be lost on restart");

                    Ok(libsql::Builder::new_local(":memory:").build().await?)
                }
                path if path.starts_with("libsql:") => {
                    let token = self.token.clone().ok_or_else(|| {
                        Error::UnknownError("libsql database requires a token".to_string())
                    })?;

                    Ok(libsql::Builder::new_remote(path.to_string(), token)
                        .build()
                        .await?)
                }
                path if path.starts_with("/")
                    || path.starts_with("./")
                    || path.starts_with("\\") =>
                {
                    log::info!("Connecting to Database: {:?}", path);
                    // Create all directories in the path
                    let dirpath = std::path::Path::new(&path);
                    if let Some(parent) = dirpath.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    Ok(libsql::Builder::new_local(path).build().await?)
                }
                _ => Err(Error::UnknownError(format!(
                    "Invalid database path: {}",
                    path
                ))),
            }
        } else {
            log::info!("Connecting to In-Memory Database");
            log::warn!("In-Memory Database is not persisted and will be lost on restart");

            Ok(libsql::Builder::new_local(":memory:").build().await?)
        }
    }

    /// Create / Connect to the Database
    pub async fn connection(&self) -> Result<libsql::Connection, Error> {
        let database = self.database().await?;
        Ok(database.connect()?)
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
