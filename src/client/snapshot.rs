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

    /// Updated Metadata
    #[serde(skip)]
    pub updated_metadata: bool,
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
        let key = key.into();
        let value = value.into();
        if !value.is_empty() {
            debug!("Adding Metadata for Snapshot({:?}) :: {}", self.id, key);
            self.metadata.insert(key, value.into());
            self.updated_metadata = true;
        } else {
            debug!("Skipping empty metadata value for key: {}", key);
        }
    }

    /// Update Metadata of the snapshot
    #[cfg(feature = "agent")]
    pub async fn update_metadata(&self, client: &KonarrClient) -> Result<(), crate::KonarrError> {
        if !self.updated_metadata {
            debug!("No metadata changes to update for Snapshot({:?})", self.id);
            return Ok(());
        }

        debug!("Updating Metadata for Snapshot({:?})", self.id);
        client
            .patch(
                format!("/snapshots/{}/metadata", self.id).as_str(),
                self.metadata.clone(),
            )
            .await?;
        Ok(())
    }

    /// Add Docker Metadata to the snapshot
    #[cfg(all(feature = "agent", feature = "docker"))]
    pub async fn add_docker(&mut self, docker: &bollard::Docker) -> Result<(), crate::KonarrError> {
        let version = docker.version().await?;

        // OS Metadata
        self.add_metadata("os", version.os.unwrap_or_default());
        self.add_metadata("os.kernel", version.kernel_version.unwrap_or_default());
        self.add_metadata("os.arch", version.arch.unwrap_or_default());
        // Container Engine
        self.add_metadata("container", "true");
        let engine = version.platform.unwrap_or_default().name;
        self.add_metadata("container.engine", engine);
        self.add_metadata(
            "container.engine.version",
            version.version.unwrap_or_default(),
        );

        Ok(())
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
