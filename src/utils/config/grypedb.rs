use std::path::PathBuf;

use super::Config;
use crate::{KonarrError as Error, utils::grypedb::GrypeDatabase};

impl Config {
    /// GrypeDB Path in data directory
    pub fn grype_path(&self) -> Result<PathBuf, Error> {
        let path = self.data_path()?.join("grypedb");
        if !path.exists() {
            log::debug!("Creating Grype path");
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Connect to a Grype Database
    pub async fn grype_connection(&self) -> Result<GrypeDatabase, Error> {
        GrypeDatabase::connect(&self.grype_path()?).await
    }
}
