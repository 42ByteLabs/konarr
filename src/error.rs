//! # Konarr Error
use thiserror::Error;

/// Konarr Error
#[derive(Error, Debug)]
pub enum KonarrError {
    /// Parsing Configuration Error
    #[error("Failed to parse the configuration file: {0}")]
    ConfigParseError(String),
    /// IO Error
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    /// Yaml Error
    #[error("IO Error: {0}")]
    YamlError(#[from] serde_yaml::Error),
    /// JSON Error
    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// Figment Error
    #[error("Figment Error")]
    FigmentError(#[from] figment::Error),
    /// Semver Version Error
    #[error("Version Error")]
    VersionError(#[from] semver::Error),

    /// Parsing Bill of Materials Error
    #[error("Failed to parse SBOM: {0}")]
    ParseSBOM(String),

    /// Indexing Data
    #[error("Failed to index data: {0}")]
    IndexingError(String),

    /// Authentication Error
    #[error("Authentication Error: {0}")]
    AuthenticationError(String),
    /// Unauthorized Error
    #[error("Unauthorized")]
    Unauthorized,

    /// GeekORM Error
    #[cfg(feature = "models")]
    #[error("GeekORM Error: {0}")]
    GeekOrm(#[from] geekorm::Error),

    /// Libsql Error
    #[cfg(feature = "models")]
    #[error("Libsql Error: {0}")]
    Libsql(#[from] libsql::Error),

    /// Tool Error
    #[cfg(feature = "tools")]
    #[error("Tool Error: {0}")]
    ToolError(String),

    /// URL Parse Error
    #[error("URL Parse Error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// Reqwest Error
    #[cfg(feature = "client")]
    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Docker / Bollard Error
    #[cfg(feature = "docker")]
    #[error("Bollard Error: {0}")]
    BollardError(#[from] bollard::errors::Error),

    /// Unknown Error
    #[error("Unknown Error: {0}")]
    UnknownError(String),
}
