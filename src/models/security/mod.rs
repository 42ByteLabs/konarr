//! # Security module
use geekorm::prelude::*;

pub mod advisories;
pub mod alerts;

pub use crate::bom::sbom::BomVulnerabilitySeverity;
pub use advisories::{Advisories, AdvisorySource};
pub use alerts::{Alerts, SecurityState};

/// Security Criticality
#[derive(Data, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum SecuritySeverity {
    /// Critical
    Critical,
    /// High
    High,
    /// Medium
    Medium,
    /// Low
    Low,
    /// Informational
    Informational,
    /// Unmantained
    Unmantained,
    /// Malware
    Malware,
    /// Unknown
    #[default]
    Unknown,
}

impl From<String> for SecuritySeverity {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "crit" | "critical" | "very-high" | "security.alerts.critical" => {
                SecuritySeverity::Critical
            }
            "high" | "security.alerts.high" => SecuritySeverity::High,
            "med" | "medium" | "moderate" | "security.alerts.medium" => SecuritySeverity::Medium,
            "low" | "security.alerts.low" => SecuritySeverity::Low,
            "info" | "information" | "informational" | "security.alerts.infomational" => {
                SecuritySeverity::Informational
            }
            "mal" | "malware" | "security.alerts.malware" => SecuritySeverity::Malware,
            "unmaintained" | "security.alerts.unmaintained" => SecuritySeverity::Unmantained,
            "unknown" | "none" | "other" | "security.alerts.unknown" | "security.alerts.other" => {
                SecuritySeverity::Unknown
            }
            _ => {
                log::warn!("Unknown Security Severity: '{}'", value);
                SecuritySeverity::Unknown
            }
        }
    }
}

impl From<&BomVulnerabilitySeverity> for SecuritySeverity {
    fn from(value: &BomVulnerabilitySeverity) -> Self {
        match value {
            BomVulnerabilitySeverity::Critical => SecuritySeverity::Critical,
            BomVulnerabilitySeverity::High => SecuritySeverity::High,
            BomVulnerabilitySeverity::Medium => SecuritySeverity::Medium,
            BomVulnerabilitySeverity::Low => SecuritySeverity::Low,
            BomVulnerabilitySeverity::Informational => SecuritySeverity::Informational,
            _ => SecuritySeverity::Unknown,
        }
    }
}

impl ToString for SecuritySeverity {
    fn to_string(&self) -> String {
        match self {
            SecuritySeverity::Critical => "Critical".to_string(),
            SecuritySeverity::High => "High".to_string(),
            SecuritySeverity::Medium => "Medium".to_string(),
            SecuritySeverity::Low => "Low".to_string(),
            SecuritySeverity::Informational => "Informational".to_string(),
            SecuritySeverity::Unmantained => "Unmantained".to_string(),
            SecuritySeverity::Malware => "Malware".to_string(),
            SecuritySeverity::Unknown => "Unknown".to_string(),
        }
    }
}
