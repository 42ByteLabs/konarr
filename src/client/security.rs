//! # Agent Security
use serde::{Deserialize, Serialize};

/// Security Summary
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySummary {
    /// Total
    pub total: u32,
    /// Critical
    pub critical: u32,
    /// High
    pub high: u32,
    /// Medium
    pub medium: u32,
    /// Low
    pub low: u32,
    /// Informational
    pub informational: u32,
    /// Unmaintained
    pub unmaintained: u32,
    /// Malware
    pub malware: u32,
    /// Unknown
    pub unknown: u32,
}
