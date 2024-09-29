//! # Bill of Materials (BOM) module

use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Bill of Materials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillOfMaterials {
    /// The type of SBOM (CycloneDX, SPDX, etc.)
    pub sbom_type: BomType,
    /// Version of the SBOM format
    pub version: String,
    /// The tool used to generate the SBOM
    pub tools: Vec<BomTool>,
    /// SHA256 of the BOM
    pub sha: String,
    /// Timestamp of the SBOM
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// The container information
    pub container: Container,
    /// The dependencies of the software
    pub components: Vec<BomComponent>,
}

impl BillOfMaterials {
    /// Create a new Bill of Materials
    pub fn new(sbom_type: BomType, version: String) -> Self {
        Self {
            sbom_type,
            version,
            tools: Vec::new(),
            sha: String::new(),
            timestamp: chrono::Utc::now(),
            container: Container::default(),
            components: Vec::new(),
        }
    }
}

/// SBOM Tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomTool {
    /// Name of the tool
    pub name: String,
    /// Version of the tool
    pub version: String,
}

/// SBOM Type Enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BomType {
    /// CycloneDX SBOM
    CycloneDX,
    /// SPDX SBOM
    SPDX,
}

/// Dependency Model
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BomComponent {
    /// Package URL
    pub purl: String,
    /// Package Name
    pub name: String,
    /// The type of component
    pub comp_type: BomComponentType,
    /// Signature of the component
    pub signature: Option<String>,
}

impl BomComponent {
    /// Create a new Dependency from a Package URL
    pub fn from_purl(purl: String) -> Self {
        // TODO: Parse the purl to get the name
        Self {
            purl,
            ..Default::default()
        }
    }
}

/// Container Information
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Container Image
    pub image: Option<String>,
    /// Container Version
    pub version: Option<String>,
    /// Container Digest
    pub image_digest: Option<String>,
    /// Container Tag
    pub image_tag: Option<String>,
}

impl Display for BomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BomType::CycloneDX => write!(f, "CycloneDX"),
            BomType::SPDX => write!(f, "SPDX"),
        }
    }
}

impl From<BomType> for String {
    fn from(value: BomType) -> Self {
        value.to_string()
    }
}

impl From<BomType> for Vec<u8> {
    fn from(value: BomType) -> Self {
        match value {
            BomType::CycloneDX => value.to_string().as_bytes().to_vec(),
            BomType::SPDX => value.to_string().as_bytes().to_vec(),
        }
    }
}

/// Component Type Enum
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum BomComponentType {
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

impl From<String> for BomComponentType {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "library" => BomComponentType::Library,
            "application" => BomComponentType::Application,
            "framework" => BomComponentType::Framework,
            "operating_system" => BomComponentType::OperatingSystem,
            "device" => BomComponentType::Device,
            "file" => BomComponentType::File,
            "container" => BomComponentType::Container,
            "firmware" => BomComponentType::Firmware,
            "data" => BomComponentType::Data,
            "service" => BomComponentType::Service,
            "database" => BomComponentType::Database,
            "operating_environment" => BomComponentType::OperatingEnvironment,
            "middleware" => BomComponentType::Middleware,
            "programming_language" => BomComponentType::ProgrammingLanguage,
            _ => BomComponentType::Unknown,
        }
    }
}
