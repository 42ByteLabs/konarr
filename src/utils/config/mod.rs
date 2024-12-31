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

use base64::Engine;
use figment::{
    providers::{Format, Serialized},
    Figment,
};
use log::{debug, warn};
use std::path::PathBuf;
use url::Url;

#[cfg(feature = "client")]
use crate::client::KonarrClient;
#[cfg(feature = "tools-grypedb")]
use crate::utils::grypedb::GrypeDatabase;
use crate::{error::KonarrError as Error, utils::rand::generate_random_string};

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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(skip)]
    path: PathBuf,

    #[serde(skip)]
    data_path: PathBuf,

    /// Database Configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Server Configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Project Configuration
    #[serde(default)]
    pub agent: AgentConfig,

    /// Sessions Configuration
    #[serde(default)]
    pub sessions: SessionsConfig,
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
            std::fs::create_dir_all(&parent)?;
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

    /// GrypeDB Path in data directory
    #[cfg(feature = "tools-grypedb")]
    pub fn grype_path(&self) -> Result<PathBuf, Error> {
        let path = self.data_path()?.join("grypedb");
        if !path.exists() {
            log::debug!("Creating Grype path");
            std::fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Connect to a Grype Database
    #[cfg(feature = "tools-grypedb")]
    pub async fn grype_connection(&self) -> Result<GrypeDatabase, Error> {
        GrypeDatabase::connect(&self.grype_path()?).await
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
    pub fn sessions<'c>(&'c self) -> &'c SessionsConfig {
        &self.sessions
    }
}

/// Database Configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Database local path
    #[serde(skip_serializing_if = "Option::is_none")]
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
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("konarr.db"),
        };

        Self { path: Some(path) }
    }
}

/// Server Configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    /// Server / Rocket Secret (default to random)
    #[serde(default)]
    pub secret: String,

    /// Server Domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    /// Scheme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    /// CORS Enabled
    #[serde(default)]
    pub cors: bool,

    // Frontend Settings
    /// Frontend static files path
    #[serde(default)]
    pub frontend: PathBuf,

    /// Entry Point for the API (default to `/api`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let frontend = match std::env::var("KONARR_CLIENT_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("client/build"),
        };

        Self {
            secret: String::new(),
            domain: None,
            port: None,
            scheme: None,
            cors: false,
            frontend,
            api: Some("/api".to_string()),
        }
    }
}

impl ServerConfig {
    /// Get the Server Configuration
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(Self::default()))
            .merge(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_SERVER_"))
    }

    /// Set Instance from URL
    pub fn set_instance(&mut self, instance: &String) -> Result<(), crate::KonarrError> {
        let url: Url = Url::parse(instance)?;

        self.scheme = Some(url.scheme().to_string());
        if let Some(host) = url.host_str() {
            self.domain = Some(host.to_string());
        }
        if let Some(port) = url.port_or_known_default() {
            self.port = Some(port as i32);
        }
        Ok(())
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
        let scheme = if let Some(scheme) = &self.scheme {
            scheme.clone()
        } else {
            "http".to_string()
        };

        let port = self
            .port
            .unwrap_or_else(|| if scheme.as_str() == "http" { 9000 } else { 443 });

        let url = Url::parse(&format!(
            "{}://{}:{}",
            scheme,
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
        let api_base = self.api.clone().unwrap_or_else(|| "/api".to_string());
        Ok(url.join(api_base.as_str())?)
    }

    /// Generate a base64 encoded secret
    pub fn generate_secret() -> String {
        log::debug!("Generating Server Secret...");
        let secret = generate_random_string(32);
        let secret64 = base64::engine::general_purpose::STANDARD.encode(secret);
        secret64
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

    /// Get the Konarr Client with Credentials
    #[cfg(feature = "client")]
    pub fn client_with_credentials(
        &self,
        username: String,
        password: String,
    ) -> Result<KonarrClient, crate::KonarrError> {
        Ok(KonarrClient::init()
            .base(self.api_url()?)?
            .credentials(username, password)
            .build()?)
    }
}

/// Sessions Configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionsConfig {
    /// Admin Config
    pub admins: SessionsRoleConfig,
    /// Users Config
    pub users: SessionsRoleConfig,
    /// Agent Config
    pub agents: SessionsRoleConfig,
}

/// Session Role Config
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    /// Agent base Project ID (default to root project of 0)
    #[serde(rename = "project-id", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<u32>,
    /// Agent Hostname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Auto-Create Projects
    #[serde(default)]
    pub create: bool,
    /// Agent Token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Monitoring Mode Enabled
    #[serde(default)]
    pub monitoring: bool,
    /// Docker Socket
    #[serde(rename = "docker-socket", skip_serializing_if = "Option::is_none")]
    pub docker_socket: Option<String>,
    /// Tool to use
    pub tool: Option<String>,
    /// Tool Auto-Install
    #[serde(default)]
    pub tool_auto_install: bool,
    /// Tool Auto-Update
    #[serde(default)]
    pub tool_auto_update: bool,
}

impl AgentConfig {
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_AGENT_"))
    }
}
