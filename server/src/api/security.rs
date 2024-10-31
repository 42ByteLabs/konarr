//! # Security API

/// Security Summary
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct SecuritySummary {
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub other: u32,
}
