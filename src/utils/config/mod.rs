//! # Konarr Configuration
//!
//! This is the main configuration for the Konarr Server. It is used to configure the server, database, and other settings.
//!
//! ## Example
//!
//! ```yaml
//! database:
//!   path: /var/lib/konarr/konarr.db
//! server:
//!   domain: localhost
//! sessions:
//!   admins:
//!     expires: 1 # 1 hour
//!   users:
//!     expires: 24 # 24 hours
//! ```
//!
//! This allows you to configure the server to run as you need it to.
//!
//!

use figment::{
    providers::{Format, Serialized},
    Figment,
};
use log::{debug, warn};
use std::path::PathBuf;
use url::Url;

#[cfg(feature = "client")]
use crate::client::KonarrClient;
use crate::error::KonarrError as Error;

/// Application Configuration
///
/// ```rust
/// let data = r#"
/// database:
///   path: /var/lib/konarr/konarr.db
/// server:
///   domain: konarr.42bytelabs.com
///   scheme: https
/// agent:
///   id: 1
/// "#;
/// // Set the KONARR_DATABASE_PATH environment variable to /etc/konarr.db
/// std::env::set_var("KONARR_DB_PATH", "/etc/konarr.db");
///
/// let config = konarr::Config::load_str(data).unwrap();
///
/// # assert_eq!(config.database.path, Some(std::path::PathBuf::from("/etc/konarr.db")));
/// # assert_eq!(config.server.domain, Some("konarr.42bytelabs.com".to_string()));
/// # assert_eq!(config.server.url().unwrap(), url::Url::parse("https://konarr.42bytelabs.com/").unwrap());
///
/// ```
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
    pub agent: AgentConfig,
}

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
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.frontend_url().unwrap();
    ///
    /// # assert_eq!(url, None);
    /// ```
    pub fn frontend_url(&self) -> Result<Option<Url>, crate::KonarrError> {
        if let Some(domain) = &self.server.domain {
            let scheme = self.server.scheme.clone();
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
        if path.exists() {
            Ok(path)
        } else {
            Err(Error::IOError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Frontend Path does not exist: {:?}", path),
            )))
        }
    }
}

/// Database Configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Database local path
    pub path: Option<PathBuf>,
}

impl DatabaseConfig {
    /// Get the Database Configuration
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_DB_"))
    }

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
    pub domain: Option<String>,
    /// Port
    pub port: Option<i32>,
    /// Scheme
    #[serde(default)]
    pub scheme: String,

    // Frontend Settings
    /// Frontend static files path
    #[serde(default)]
    pub frontend: PathBuf,

    /// Entry Point for the API (default to `/api`)
    #[serde(default)]
    pub api: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let frontend = match std::env::var("KONARR_CLIENT_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("client/build"),
        };

        Self {
            domain: None,
            port: None,
            scheme: "http".to_string(),
            frontend,
            api: "/api".to_string(),
        }
    }
}

impl ServerConfig {
    /// Get the Server Configuration
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_SERVER_"))
    }
    /// Get the Server URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.server.url().unwrap();
    ///
    /// assert_eq!(url.as_str(), "http://localhost:9000/");
    /// ```
    pub fn url(&self) -> Result<Url, crate::KonarrError> {
        let port = if self.scheme.as_str() == "http" {
            9000
        } else {
            443
        };

        let url = Url::parse(&format!(
            "{}://{}:{}",
            self.scheme.clone(),
            self.domain.clone().unwrap_or("localhost".to_string()),
            port
        ))?;
        if url.scheme() != "https" {
            warn!("Using insecure scheme: {}", url.scheme());
        }
        Ok(url)
    }
    /// Get the Server API URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.server.api_url().unwrap();
    ///
    /// assert_eq!(url.as_str(), "http://localhost:9000/api");
    /// ```
    pub fn api_url(&self) -> Result<Url, crate::KonarrError> {
        let url = self.url()?;
        Ok(url.join(self.api.as_str())?)
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
pub struct AgentConfig {
    /// Agent base Project ID (default to root project of 0)
    pub id: Option<u32>,

    /// Agent Token
    pub token: Option<String>,
}

impl AgentConfig {
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_AGENT_"))
    }
}
