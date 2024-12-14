//! # Bill of Materials (BOM) module

use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
    /// List of vulnerabilities
    pub vulnerabilities: Vec<BomVulnerability>,
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
            vulnerabilities: Vec::new(),
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
#[allow(non_camel_case_types)]
pub enum BomType {
    /// CycloneDX v1.5
    CycloneDX_1_5,
    /// CycloneDX v1.6
    CycloneDX_1_6,
    /// SPDX SBOM
    SPDX,
}

impl Display for BomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BomType::CycloneDX_1_5 => write!(f, "CycloneDX v1.5"),
            BomType::CycloneDX_1_6 => write!(f, "CycloneDX v1.6"),
            BomType::SPDX => write!(f, "SPDX"),
        }
    }
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

/// Bill of Materials Vulnerability
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BomVulnerability {
    /// CVE/GHA/etc.
    pub name: String,
    /// Advisory Source
    pub source: String,
    /// Severity of the vulnerability
    pub severity: BomVulnerabilitySeverity,
    /// Description of the vulnerability
    pub description: Option<String>,
    /// URL to the advisory
    pub url: Option<String>,
    /// Affects packages
    pub components: Vec<BomComponent>,
}

impl BomVulnerability {
    /// Create a new BOM Vulnerability
    pub fn new(name: String, source: String, severity: String) -> Self {
        BomVulnerability {
            name,
            source,
            severity: BomVulnerabilitySeverity::from(severity),
            ..Default::default()
        }
    }
}

/// Vulnerability Severity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum BomVulnerabilitySeverity {
    /// Critical severity
    Critical,
    /// High severity
    High,
    /// Medium severity
    Medium,
    /// Low severity
    Low,
    /// Informational severity
    Informational,
    /// Unknown severity
    #[default]
    Unknown,
}

impl From<String> for BomVulnerabilitySeverity {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "critical" | "very-high" => BomVulnerabilitySeverity::Critical,
            "high" => BomVulnerabilitySeverity::High,
            "medium" | "moderate" => BomVulnerabilitySeverity::Medium,
            "low" => BomVulnerabilitySeverity::Low,
            "informational" | "very-low" => BomVulnerabilitySeverity::Informational,
            _ => BomVulnerabilitySeverity::Unknown,
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

impl From<BomType> for String {
    fn from(value: BomType) -> Self {
        value.to_string()
    }
}

impl From<BomType> for Vec<u8> {
    fn from(value: BomType) -> Self {
        match value {
            BomType::CycloneDX_1_5 => value.to_string().as_bytes().to_vec(),
            BomType::CycloneDX_1_6 => value.to_string().as_bytes().to_vec(),
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
    /// Container
    Container,
    /// Firmware
    Firmware,
    /// CryptoLibrary
    CryptoLibrary,
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
            "container" => BomComponentType::Container,
            "firmware" => BomComponentType::Firmware,
            "service" => BomComponentType::Service,
            "database" => BomComponentType::Database,
            "operating_environment" => BomComponentType::OperatingEnvironment,
            "middleware" => BomComponentType::Middleware,
            "programming_language" => BomComponentType::ProgrammingLanguage,
            _ => BomComponentType::Unknown,
        }
    }
}
