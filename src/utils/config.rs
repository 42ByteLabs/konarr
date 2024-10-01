//! Configuration

use std::path::PathBuf;

use log::{debug, warn};
use url::Url;

#[cfg(feature = "client")]
use crate::client::KonarrClient;
use crate::error::KonarrError as Error;

/// Application Configuration
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Database Configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Server Configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Project Configuration
    #[serde(default)]
    pub project: ProjectConfig,

    /// Frontend Configuration
    #[serde(default)]
    pub frontend: FrontendConfig,

    /// Session Configuration
    #[serde(default)]
    pub sessions: SessionsConfig,
}

impl Config {
    /// Load the Configuration
    pub fn load(path: &PathBuf) -> Result<Self, Error> {
        debug!("Loading Configuration: {:?}", path);
        let config = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(config.as_str())?)
    }

    /// Save the Configuration
    pub fn save(&self, path: &PathBuf) -> Result<(), Error> {
        debug!("Saving Configuration: {:?}", path);
        let config = serde_yaml::to_string(self)?;
        std::fs::write(path, config)?;
        Ok(())
    }

    #[cfg(feature = "models")]
    /// Get Database Connection
    pub async fn database(&self) -> Result<libsql::Database, Error> {
        self.database.database().await
    }

    /// Get Frontend URL
    pub fn frontend_url(&self) -> Result<Url, crate::KonarrError> {
        if let Some(domain) = &self.frontend.domain {
            Ok(domain.clone())
        } else {
            Ok(Url::parse(
                format!(
                    "{}://{}:{}",
                    self.server.scheme.clone().unwrap_or("http".to_string()),
                    self.server.domain,
                    self.server.port
                )
                .as_str(),
            )?)
        }
    }
    /// Get the Frontend Path
    pub fn frontend_path(&self) -> Result<PathBuf, Error> {
        let path = self.frontend.path.clone();
        if path.exists() {
            Ok(path)
        } else {
            Err(Error::IOError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Frontend Path does not exist: {:?}", path),
            )))
        }
    }

    /// Get Sessions Configuration
    pub fn sessions<'c>(&'c self) -> &'c SessionsConfig {
        &self.sessions
    }
}

/// Database Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Database local path
    pub path: Option<PathBuf>,
}

impl DatabaseConfig {
    /// Create / Connect to the Database
    #[cfg(feature = "models")]
    pub async fn database(&self) -> Result<libsql::Database, Error> {
        if let Some(path) = &self.path {
            log::info!("Connecting to Database: {:?}", path);

            // Create all directories in the path
            let dirpath = std::path::Path::new(&path);
            if let Some(parent) = dirpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            Ok(libsql::Builder::new_local(path).build().await?)
        } else {
            log::info!("Connecting to In-Memory Database");
            Ok(libsql::Builder::new_local(":memory:").build().await?)
        }
    }

    /// Create / Connect to the Database
    #[cfg(feature = "models")]
    pub async fn connection(&self) -> Result<libsql::Connection, Error> {
        let database = self.database().await?;
        Ok(database.connect()?)
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let path = match std::env::var("KONARR_DATABASE_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("konarr.db"),
        };

        Self { path: Some(path) }
    }
}

/// Server Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    /// Server Domain
    pub domain: String,
    /// Port
    pub port: u16,
    /// Scheme
    #[serde(default)]
    pub scheme: Option<String>,
    /// Entry Point
    #[serde(default)]
    pub api: ServerApiConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            domain: "localhost".to_string(),
            port: 9000,
            scheme: Some("https".to_string()),
            api: ServerApiConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Get the Server URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.server.url().unwrap();
    ///
    /// assert_eq!(url.as_str(), "https://localhost:9000/");
    /// ```
    pub fn url(&self) -> Result<Url, crate::KonarrError> {
        let url = Url::parse(&format!(
            "{}://{}:{}",
            self.scheme.clone().unwrap_or("https".to_string()),
            self.domain,
            self.port
        ))?;
        if url.scheme() != "https" {
            warn!("Using insecure scheme: {}", url.scheme());
        }
        Ok(url)
    }
    /// Get the Server API URL
    pub fn api_url(&self) -> Result<Url, crate::KonarrError> {
        let url = self.url()?;
        Ok(url.join(self.api.entrypoint.as_str())?)
    }

    /// Get the Konarr Client
    #[cfg(feature = "client")]
    pub fn client(&self) -> Result<KonarrClient, crate::KonarrError> {
        Ok(KonarrClient::init().base(self.api_url()?)?.build()?)
    }

    /// Get the Konarr Client with Token
    #[cfg(feature = "client")]
    pub fn client_with_token(&self, token: String) -> Result<KonarrClient, crate::KonarrError> {
        Ok(KonarrClient::init()
            .base(self.api_url()?)?
            .token(token)
            .build()?)
    }
}

/// Server API Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ServerApiConfig {
    /// Entry Point
    pub entrypoint: String,
}

impl Default for ServerApiConfig {
    fn default() -> Self {
        Self {
            entrypoint: "/api".to_string(),
        }
    }
}

/// Frontend Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FrontendConfig {
    /// Path to the Frontend Files to serve
    pub path: PathBuf,
    /// Domain of the Frontend (for CORS)
    pub domain: Option<Url>,
}

impl Default for FrontendConfig {
    fn default() -> Self {
        let path = match std::env::var("KONARR_CLIENT_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("client/build"),
        };

        Self { path, domain: None }
    }
}

/// Sessions Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SessionsConfig {
    /// Admin Config
    pub admins: SessionsRoleConfig,
    /// Users Config
    pub users: SessionsRoleConfig,
    /// Agent Config
    pub agents: SessionsRoleConfig,
}

/// Session Role Config
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SessionsRoleConfig {
    /// Time in hours of when a session should be invalidated
    pub expires: i32,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            admins: SessionsRoleConfig { expires: 1 as i32 },
            users: SessionsRoleConfig { expires: 24 as i32 },
            agents: SessionsRoleConfig {
                expires: 720 as i32,
            },
        }
    }
}

/// Project Configuration
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    /// Project ID
    pub id: Option<u32>,
}
