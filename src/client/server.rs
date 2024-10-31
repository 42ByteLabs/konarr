//! Server Information
use serde::{Deserialize, Serialize};

/// Server Information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// Server Version
    pub version: String,
    /// Server Commit Hash
    pub commit: String,
    /// Server Configuration
    pub config: Option<ServerConfig>,
    /// Server User
    pub user: Option<User>,
}

/// Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Is Server Initialised
    pub initialised: bool,
    /// Is Server Registration Enabled
    pub registration: bool,
}

/// User Information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// Username
    pub username: String,
    /// User Role
    pub role: String,
    /// User Avatar
    pub avatar: Option<String>,
}
