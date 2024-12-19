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
    /// Summary of Konarr Projects
    pub projects: Option<ProjectsSummary>,
    /// Summary of Dependencies
    pub dependencies: Option<DependencySummary>,
    /// Security Summary
    pub security: Option<SecuritySummary>,
    /// Agent Settings
    pub agent: Option<AgentSettings>,
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

/// Konarr Project Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsSummary {
    /// Total Projects
    pub total: u32,
    /// Servers
    pub servers: u32,
    /// Containers
    pub containers: u32,
}

/// Dependency Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencySummary {
    /// Total Dependencies
    pub total: u32,
    /// Libraries
    pub libraries: u32,
    /// Frameworks
    pub frameworks: u32,
    /// Operating Systems
    #[serde(rename = "operating-systems")]
    pub operating_systems: u32,
    /// Programming Languages
    pub languages: u32,
    #[serde(rename = "package-managers")]
    /// Package Managers
    pub package_managers: u32,
    /// Compression Libraries
    #[serde(rename = "compression-libraries")]
    pub compression_libraries: u32,
    /// Cryptographic Libraries
    #[serde(rename = "cryptographic-libraries")]
    pub cryptographic_libraries: u32,
    /// Database (Application or Libraries)
    pub databases: u32,
    /// Operating Environments
    #[serde(rename = "operating-environments")]
    pub operating_environments: u32,
    /// Middleware
    pub middleware: u32,
}

/// Security Summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySummary {
    /// Total Security Issues
    pub total: u32,
    /// Critical Security Issues
    pub critical: u32,
    /// High Security Issues
    pub high: u32,
    /// Medium Security Issues
    pub medium: u32,
    /// Low Security Issues
    pub low: u32,
    /// Informational Security Issues
    pub informational: u32,
    /// Malware Security Issues
    pub malware: u32,
    /// Unmaintained Security Issues
    pub unmaintained: u32,
    /// Unknown Security Issues
    pub unknown: u32,
}

/// Agent Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettings {
    /// Agent tool name
    pub tool: String,
}
