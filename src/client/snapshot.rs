//! Snapshot Request
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::security::SecuritySummary;
use super::{ApiResponse, KonarrClient};

/// Snapshot Request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KonarrSnapshot {
    /// Snapshot ID
    pub id: u32,
    /// Dependencies Count
    pub dependencies: u32,
    /// Security Summary
    #[serde(default)]
    pub security: SecuritySummary,
    /// Snapshot Metadata
    pub metadata: HashMap<String, String>,
    /// Created At
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl KonarrSnapshot {
    /// Create a new snapshot
    pub async fn create(
        client: &KonarrClient,
        project_id: u32,
    ) -> Result<ApiResponse<Self>, crate::KonarrError> {
        debug!("Creating snapshot for project `{}`", project_id);
        Ok(client
            .post(
                "/snapshots",
                serde_json::json!({
                    "project_id": project_id,
                }),
            )
            .await?
            .json::<ApiResponse<Self>>()
            .await?)
    }

    /// Get a snapshot by ID
    pub async fn by_id(
        client: &KonarrClient,
        snapshot_id: u32,
    ) -> Result<KonarrSnapshot, crate::KonarrError> {
        Ok(client
            .get(format!("/snapshots/{}", snapshot_id).as_str())
            .await?
            .json()
            .await?)
    }

    /// Update Metadata to a snapshot (only update on changes)
    pub async fn update_metadata(
        &self,
        client: &KonarrClient,
        data: HashMap<&str, String>,
    ) -> Result<(), crate::KonarrError> {
        let mut changes = HashMap::new();

        for (key, value) in data {
            if (self.metadata.contains_key(key) && self.metadata[key] != value)
                || !self.metadata.contains_key(key)
            {
                changes.insert(key, value);
            }
        }

        if changes.len() == 0 {
            return Ok(());
        }

        debug!("Updating Metadata for Snapshot({:?})", self.id);
        client
            .patch(format!("/snapshots/{}/metadata", self.id).as_str(), changes)
            .await?;
        Ok(())
    }

    /// Upload BOM to the the snapshot
    pub async fn upload_bom<T>(
        &self,
        client: &KonarrClient,
        data: T,
    ) -> Result<(), crate::KonarrError>
    where
        T: Serialize + Send,
    {
        debug!("Uploading BOM for Snapshot({:?})", self.id);
        client
            .post(format!("/snapshots/{}/bom", self.id).as_str(), data)
            .await?;
        Ok(())
    }
}
