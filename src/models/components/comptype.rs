//! # Component Type
use geekorm::Data;

use crate::bom::sbom::BomComponentType;

/// Component Type Enum
#[derive(Data, Debug, Hash, Default, Clone, PartialEq, Eq)]
#[geekorm(from_string = "lowercase")]
pub enum ComponentType {
    /// Library
    #[geekorm(aliases = "library,lib")]
    Library,
    /// Application
    #[geekorm(aliases = "application,app")]
    Application,
    /// Framework
    #[geekorm(aliases = "framework")]
    Framework,
    /// Operating System
    #[geekorm(aliases = "os,operatingsystem,operating_system")]
    OperatingSystem,
    /// Package Manager
    #[geekorm(aliases = "packagemanager,package_manager,package-manager")]
    PackageManager,
    /// Container
    #[geekorm(aliases = "container,docker")]
    Container,
    /// Firmware
    #[geekorm(aliases = "firmware")]
    Firmware,
    /// Cryptograph Library
    #[geekorm(aliases = "cryptographylibrary,crypto,cryptography,cryptography_library")]
    CryptographyLibrary,
    /// Service
    #[geekorm(aliases = "service")]
    Service,
    /// Database
    #[geekorm(aliases = "db,database")]
    Database,
    /// Compression Library
    #[geekorm(aliases = "compression,compressionlibrary,compression_library")]
    CompressionLibrary,
    /// Operating Environment
    #[geekorm(aliases = "oe,operatingenvironment,operating_environment")]
    OperatingEnvironment,
    /// Middleware
    #[geekorm(aliases = "middleware")]
    Middleware,
    /// Programming Language
    #[geekorm(aliases = "language,programminglanguage,programming_language")]
    ProgrammingLanguage,
    /// Unknown
    #[default]
    #[geekorm(aliases = "unknown")]
    Unknown,
}

impl From<BomComponentType> for ComponentType {
    fn from(value: BomComponentType) -> Self {
        match value {
            BomComponentType::Library => ComponentType::Library,
            BomComponentType::Application => ComponentType::Application,
            BomComponentType::Framework => ComponentType::Framework,
            BomComponentType::OperatingSystem => ComponentType::OperatingSystem,
            BomComponentType::Container => ComponentType::Container,
            BomComponentType::Firmware => ComponentType::Firmware,
            BomComponentType::CryptoLibrary => ComponentType::CryptographyLibrary,
            BomComponentType::Service => ComponentType::Service,
            BomComponentType::Database => ComponentType::Database,
            BomComponentType::OperatingEnvironment => ComponentType::OperatingEnvironment,
            BomComponentType::Middleware => ComponentType::Middleware,
            BomComponentType::ProgrammingLanguage => ComponentType::ProgrammingLanguage,
            BomComponentType::Unknown => ComponentType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        for lib in ["library", "lib", "LiBrArY", "LIB"] {
            let plib = ComponentType::from(lib);
            assert_eq!(plib, ComponentType::Library);
            assert_eq!(plib.to_string(), "Library");
        }
        for app in ["application", "APP"] {
            let papp = ComponentType::from(app);
            assert_eq!(papp, ComponentType::Application);
            assert_eq!(papp.to_string(), "Application");
        }
        for crypto in ["CrYpTo", "cryptography", "cryptography_library"] {
            let pcrypto = ComponentType::from(crypto);
            assert_eq!(pcrypto, ComponentType::CryptographyLibrary);
            assert_eq!(pcrypto.to_string(), "CryptographyLibrary");
        }
    }
}
