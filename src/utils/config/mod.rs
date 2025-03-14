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

use figment::{Figment, providers::Serialized};
use std::path::PathBuf;

#[cfg(feature = "client")]
mod client;
mod config;
#[cfg(feature = "tools-grypedb")]
mod grypedb;
#[cfg(feature = "models")]
mod models;
mod server;

/// Application Configuration
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

/// Database Configuration
///
/// All database related settings are stored in this struct.
///
/// Settings are loaded from the `KONARR_DB_` environment variables.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Database path. This can be a file path or a connection string.
    ///
    /// If the path or environment variable is not set, it will default to an in-memory database.
    ///
    /// Env: `KONARR_DB_PATH`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Database auth token, password or key
    ///
    /// This is used for secure communication between the server and the database.
    ///
    /// Env: `KONARR_DB_TOKEN`
    pub token: Option<String>,
}

impl DatabaseConfig {
    /// Get the Database Configuration
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_DB_"))
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        //
        let path = if let Ok(p) = std::env::var("KONARR_DB_PATH") {
            Some(p)
        } else if let Ok(data_path) = std::env::var("KONARR_DATA_PATH") {
            // If the data path is set, use it to build the database path
            let path = PathBuf::from(data_path).join("konarr.db");
            Some(path.to_string_lossy().to_string())
        } else {
            None
        };

        Self { path, token: None }
    }
}

/// Server Configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerConfig {
    /// Server / Rocket Secret (default to random)
    ///
    /// Env: `KONARR_SERVER_SECRET`
    #[serde(default)]
    pub secret: String,

    /// Server Domain
    ///
    /// Env: `KONARR_SERVER_DOMAIN`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Port
    ///
    /// Env: `KONARR_SERVER_PORT`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    /// Scheme
    ///
    /// Env: `KONARR_SERVER_SCHEME`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    /// CORS Enabled
    ///
    /// Env: `KONARR_SERVER_CORS`
    #[serde(default)]
    pub cors: bool,

    // Frontend Settings
    /// Frontend static files path
    ///
    /// Env: `KONARR_CLIENT_PATH`
    #[serde(default)]
    pub frontend: PathBuf,

    /// Entry Point for the API (default to `/api`)
    ///
    /// Env: `KONARR_SERVER_API`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let frontend = match std::env::var("KONARR_CLIENT_PATH") {
            Ok(path) => PathBuf::from(path),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("frontend")
                .join("build"),
        };

        Self {
            secret: String::new(),
            domain: None,
            port: None,
            scheme: None,
            cors: true,
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
    ///
    /// Env: `KONARR_AGENT_PROJECT_ID`
    #[serde(rename = "project-id", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<u32>,
    /// Agent Hostname
    ///
    /// Env: `KONARR_AGENT_HOST`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Auto-Create Projects
    ///
    /// Env: `KONARR_AGENT_CREATE`
    #[serde(default)]
    pub create: bool,
    /// Agent Token
    ///
    /// Env: `KONARR_AGENT_TOKEN`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Monitoring Mode Enabled
    ///
    /// Env: `KONARR_AGENT_MONITORING`
    #[serde(default)]
    pub monitoring: bool,
    /// Docker Socket
    ///
    /// Env: `KONARR_AGENT_DOCKER_SOCKET`
    #[serde(rename = "docker-socket", skip_serializing_if = "Option::is_none")]
    pub docker_socket: Option<String>,
    /// Tool to use
    ///
    /// Env: `KONARR_AGENT_TOOL`
    pub tool: Option<String>,
    /// Tool Auto-Install
    ///
    /// Env: `KONARR_AGENT_TOOL_AUTO_INSTALL`
    #[serde(default)]
    pub tool_auto_install: bool,
    /// Tool Auto-Update
    ///
    /// Env: `KONARR_AGENT_TOOL_AUTO_UPDATE`
    #[serde(default)]
    pub tool_auto_update: bool,
}

impl AgentConfig {
    pub(crate) fn figment(base: &Self) -> Figment {
        Figment::from(Serialized::defaults(base))
            .merge(figment::providers::Env::prefixed("KONARR_AGENT_"))
    }
}
