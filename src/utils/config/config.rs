use super::{AgentConfig, Config, DatabaseConfig, ServerConfig, SessionsConfig};
use crate::error::KonarrError as Error;
use figment::{Figment, providers::Format};
use log::debug;
use std::path::PathBuf;
use url::Url;

impl Config {
    /// Load the Configuration
    pub fn load(path: &PathBuf) -> Result<Self, Error> {
        debug!("Loading Configuration: {:?}", path);

        let figment = Figment::new()
            .merge(figment::providers::Yaml::file(path))
            .merge(figment::providers::Env::prefixed("KONARR_"));

        let mut config: Self = figment.extract()?;
        // TODO: Redo this to be more dynamic
        config.database = DatabaseConfig::figment(&config.database).extract()?;
        config.server = ServerConfig::figment(&config.server).extract()?;
        config.agent = AgentConfig::figment(&config.agent).extract()?;

        // Generate a secret if one is not provided
        if config.server.secret.is_empty() {
            config.server.secret = ServerConfig::generate_secret();
        }
        // Set the data path
        if std::env::var("KONARR_DATA_PATH").is_ok() {
            config.data_path = PathBuf::from(std::env::var("KONARR_DATA_PATH").unwrap());
        } else {
            config.data_path = PathBuf::from("./data");
        }
        config.path = path.clone();

        debug!("Finished Loading Configuration");
        Ok(config)
    }

    /// Load the Configuration from a String
    pub fn load_str(data: impl Into<String>) -> Result<Self, Error> {
        let data = data.into();
        debug!("Loading Configuration from str");

        let figment = Figment::new()
            .merge(figment::providers::Yaml::string(&data))
            .merge(figment::providers::Env::prefixed("KONARR_"));

        let mut config: Self = figment.extract()?;
        config.database = DatabaseConfig::figment(&config.database).extract()?;
        config.server = ServerConfig::figment(&config.server).extract()?;
        config.agent = AgentConfig::figment(&config.agent).extract()?;
        Ok(config)
    }

    /// Save the Configuration
    pub fn save(&self, path: &PathBuf) -> Result<(), Error> {
        debug!("Saving Configuration: {:?}", path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let config = serde_yaml::to_string(self)?;
        std::fs::write(path, config)?;
        Ok(())
    }

    /// Automatically save the Configuration
    pub fn autosave(&self) -> Result<(), Error> {
        self.save(&self.path)
    }

    /// Config directory path
    pub fn config_path(&self) -> Result<PathBuf, Error> {
        Ok(self
            .path
            .parent()
            .ok_or_else(|| Error::ConfigParseError("Invalid Config Path".to_string()))?
            .to_path_buf())
    }

    /// Data directory path
    pub fn data_path(&self) -> Result<&PathBuf, Error> {
        if !self.data_path.exists() {
            log::debug!("Creating data path");
            std::fs::create_dir_all(&self.data_path)?;
        }
        Ok(&self.data_path)
    }

    /// SBOMs Path in data directory
    pub fn sboms_path(&self) -> Result<PathBuf, Error> {
        let path = self.data_path()?.join("sboms");
        if !path.exists() {
            log::debug!("Creating SBOMs path");
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Get Frontend URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.frontend_url().unwrap();
    ///
    /// # assert_eq!(url, None);
    /// ```
    pub fn frontend_url(&self) -> Result<Option<Url>, crate::KonarrError> {
        if let Some(domain) = &self.server.domain {
            let scheme = self
                .server
                .scheme
                .clone()
                .unwrap_or_else(|| "http".to_string());

            if scheme.as_str() == "http" {
                log::warn!("Insecure HTTP is being used...")
            }

            let url_str = if let Some(port) = self.server.port {
                format!("{}://{}:{}", scheme, domain, port)
            } else {
                format!("{}://{}", scheme, domain)
            };

            Ok(Some(Url::parse(&url_str)?))
        } else {
            Ok(None)
        }
    }
    /// Get the Frontend Path
    pub fn frontend_path(&self) -> Result<PathBuf, Error> {
        let path = self.server.frontend.clone();
        if !path.exists() {
            log::debug!("Creating frontend path");
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Get Sessions Configuration
    pub fn sessions(&self) -> &SessionsConfig {
        &self.sessions
    }
}
