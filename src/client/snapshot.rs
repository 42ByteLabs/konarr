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
    pub security: Option<SecuritySummary>,
    /// Snapshot Metadata
    pub metadata: HashMap<String, String>,
    /// Created At
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// If the snapshot is new
    #[serde(skip)]
    pub new: bool,
}

impl KonarrSnapshot {
    /// Create a new snapshot
    pub async fn create(
        client: &KonarrClient,
        project_id: u32,
    ) -> Result<Self, crate::KonarrError> {
        debug!("Creating snapshot for project `{}`", project_id);
        match client
            .post(
                "/snapshots",
                serde_json::json!({
                    "project_id": project_id,
                }),
            )
            .await?
            .json::<ApiResponse<Self>>()
            .await?
        {
            ApiResponse::Ok(mut snapshot) => {
                snapshot.new = true;
                Ok(snapshot)
            }
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Get a snapshot by ID
    pub async fn by_id(
        client: &KonarrClient,
        snapshot_id: u32,
    ) -> Result<Self, crate::KonarrError> {
        debug!("Getting snapshot by ID: `{}`", snapshot_id);
        match client
            .get(format!("/snapshots/{}", snapshot_id).as_str())
            .await?
            .json::<ApiResponse<Self>>()
            .await?
        {
            ApiResponse::Ok(snapshot) => Ok(snapshot),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Add Metadata to the current snapshot
    #[cfg(feature = "agent")]
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Update Metadata of the snapshot
    #[cfg(feature = "agent")]
    pub async fn update_metadata(
        &self,
        client: &KonarrClient,
    ) -> Result<Vec<KonarrSnapshot>, crate::KonarrError> {
        debug!("Updating Metadata for Snapshot({:?})", self.id);
        match client
            .patch(
                format!("/snapshots/{}/metadata", self.id).as_str(),
                self.metadata.clone(),
            )
            .await?
            .json::<ApiResponse<Vec<Self>>>()
            .await?
        {
            ApiResponse::Ok(snapshot) => Ok(snapshot),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Update Metadata to a snapshot (only update on changes)
    #[cfg(not(feature = "agent"))]
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
    ) -> Result<Self, crate::KonarrError>
    where
        T: Serialize + Send,
    {
        debug!("Uploading BOM for Snapshot({:?})", self.id);

        match client
            .post(format!("/snapshots/{}/bom", self.id).as_str(), data)
            .await?
            .json::<ApiResponse<Self>>()
            .await?
        {
            ApiResponse::Ok(snapshot) => Ok(snapshot),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }
}
