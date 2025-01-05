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
    /// Parsing PURL
    #[error("PURL parsing error")]
    PurlError(#[from] purl::ParseError),

    /// Indexing Data
    #[error("Failed to index data: {0}")]
    IndexingError(String),
    /// Invalid Data
    #[error("Invalid Data: {0}")]
    InvalidData(String),

    /// Authentication Error
    #[error("Authentication Error: {0}")]
    AuthenticationError(String),
    /// Unauthorized Error
    #[error("Unauthorized")]
    Unauthorized,

    /// KonarrClient API Error
    #[cfg(feature = "client")]
    #[error("KonarrClient API Error: {0}")]
    KonarrClient(String),

    /// GeekORM Error
    #[cfg(feature = "models")]
    #[error("{0}")]
    GeekOrm(#[from] geekorm::Error),

    /// Libsql Error
    #[cfg(feature = "models")]
    #[error("{0}")]
    Libsql(#[from] libsql::Error),

    /// Tool Error
    #[cfg(feature = "tools")]
    #[error("Tool Error: {0}")]
    ToolError(String),

    /// From Utf8 Error
    #[error("{0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    /// Error parsing a datetime
    #[error("{0}")]
    ParseDateTimeError(#[from] chrono::ParseError),

    /// URL Parse Error
    #[error("{0}")]
    UrlParseError(#[from] url::ParseError),

    /// Reqwest Error
    #[cfg(feature = "client")]
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Docker / Bollard Error
    #[cfg(feature = "docker")]
    #[error("{0}")]
    BollardError(#[from] bollard::errors::Error),

    /// Unknown Error
    #[error("Unknown Error: {0}")]
    UnknownError(String),
}

#[cfg(feature = "client")]
impl From<crate::client::ApiError> for KonarrError {
    fn from(error: crate::client::ApiError) -> Self {
        if let Some(details) = error.details {
            KonarrError::KonarrClient(format!("{} - {}", error.message, details))
        } else {
            KonarrError::KonarrClient(error.message)
        }
    }
}
