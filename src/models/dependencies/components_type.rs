//! # Component Type
use geekorm::Data;
use std::fmt::Display;

use crate::bom::sbom::BomComponentType;

/// Component Type Enum
#[derive(Data, Debug, Default, Clone)]
pub enum ComponentType {
    /// Library
    Library,
    /// Application
    Application,
    /// Framework
    Framework,
    /// Operating System
    OperatingSystem,
    /// Device
    Device,
    /// File
    File,
    /// Container
    Container,
    /// Firmware
    Firmware,
    /// Data
    Data,
    /// Service
    Service,
    /// Database
    Database,
    /// Operating Environment
    OperatingEnvironment,
    /// Middleware
    Middleware,
    /// Programming Language
    ProgrammingLanguage,
    /// Unknown
    #[default]
    Unknown,
}

impl Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentType::Library => write!(f, "library"),
            ComponentType::Application => write!(f, "application"),
            ComponentType::Framework => write!(f, "framework"),
            ComponentType::OperatingSystem => write!(f, "operating_system"),
            ComponentType::Device => write!(f, "device"),
            ComponentType::File => write!(f, "file"),
            ComponentType::Container => write!(f, "container"),
            ComponentType::Firmware => write!(f, "firmware"),
            ComponentType::Data => write!(f, "data"),
            ComponentType::Service => write!(f, "service"),
            ComponentType::Database => write!(f, "database"),
            ComponentType::OperatingEnvironment => write!(f, "operating_environment"),
            ComponentType::Middleware => write!(f, "middleware"),
            ComponentType::ProgrammingLanguage => write!(f, "programming_language"),
            ComponentType::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&String> for ComponentType {
    fn from(value: &String) -> Self {
        match value.to_lowercase().as_str() {
            "lib" | "library" => ComponentType::Library,
            "application" => ComponentType::Application,
            "framework" => ComponentType::Framework,
            "os" | "operatingsystem" | "operating_system" => ComponentType::OperatingSystem,
            "device" => ComponentType::Device,
            "file" => ComponentType::File,
            "container" => ComponentType::Container,
            "firmware" => ComponentType::Firmware,
            "data" => ComponentType::Data,
            "service" => ComponentType::Service,
            "db" | "database" => ComponentType::Database,
            "operatingenvironment" | "operating_environment" => ComponentType::OperatingEnvironment,
            "middleware" => ComponentType::Middleware,
            "programminglanguage" | "programming_language" => ComponentType::ProgrammingLanguage,
            _ => ComponentType::Unknown,
        }
    }
}

impl From<BomComponentType> for ComponentType {
    fn from(value: BomComponentType) -> Self {
        match value {
            BomComponentType::Library => ComponentType::Library,
            BomComponentType::Application => ComponentType::Application,
            BomComponentType::Framework => ComponentType::Framework,
            BomComponentType::OperatingSystem => ComponentType::OperatingSystem,
            BomComponentType::Device => ComponentType::Device,
            BomComponentType::File => ComponentType::File,
            BomComponentType::Container => ComponentType::Container,
            BomComponentType::Firmware => ComponentType::Firmware,
            BomComponentType::Data => ComponentType::Data,
            BomComponentType::Service => ComponentType::Service,
            BomComponentType::Database => ComponentType::Database,
            BomComponentType::OperatingEnvironment => ComponentType::OperatingEnvironment,
            BomComponentType::Middleware => ComponentType::Middleware,
            BomComponentType::ProgrammingLanguage => ComponentType::ProgrammingLanguage,
            BomComponentType::Unknown => ComponentType::Unknown,
        }
    }
}
