//! # Security module

use geekorm::prelude::*;

pub mod advisories;
pub mod alerts;

pub use crate::bom::sbom::BomVulnerabilitySeverity;
pub use advisories::{Advisories, AdvisorySource};
pub use alerts::{Alerts, SecurityState};

/// List of Security Criticality
pub const SECURITY_SEVERITY: [&'static str; 8] = [
    "Critical",
    "High",
    "Medium",
    "Low",
    "Informational",
    "Unmantained",
    "Malware",
    "Unknown",
];

/// Security Criticality
#[derive(Data, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum SecuritySeverity {
    /// Critical
    #[geekorm(aliases = "critical,crit,very-high,security.alerts.critical")]
    Critical,
    /// High
    #[geekorm(aliases = "high,security.alerts.high")]
    High,
    /// Medium
    #[geekorm(aliases = "medium,med,moderate,security.alerts.medium")]
    Medium,
    /// Low
    #[geekorm(aliases = "low,security.alerts.low")]
    Low,
    /// Informational
    #[geekorm(aliases = "informational,info,security.alerts.informational")]
    Informational,
    /// Unmantained
    #[geekorm(aliases = "unmaintained,security.alerts.unmaintained")]
    Unmantained,
    /// Malware
    #[geekorm(aliases = "mal,security.alerts.malware")]
    Malware,
    /// Unknown
    #[geekorm(aliases = "unknown,none,other,security.alerts.unknown,security.alerts.other")]
    #[default]
    Unknown,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        let criticals = vec![
            "Critical",
            "critical",
            "crit",
            "very-high",
            "security.alerts.critical",
        ];
        for crit in criticals {
            assert_eq!(SecuritySeverity::from(crit), SecuritySeverity::Critical);
        }
    }
}
