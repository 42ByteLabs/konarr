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
    /// Other
    pub other: u32,
}
