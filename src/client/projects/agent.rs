//! # Konarr Project - Agent
use super::KonarrProject;
use crate::{
    KonarrClient, KonarrError,
    client::{ServerInfo, snapshot::KonarrSnapshot},
};
use log::{debug, info};

/// Konarr Project Snapshot Data struct
///
/// This struct is used to store a different data for checking if
/// a snapshot needs to be created
#[derive(Debug, Default)]
pub struct KonarrProjectSnapshotData {
    /// Server Information
    pub info: Option<ServerInfo>,
    /// Container SHA 256
    pub container_sha: Option<String>,
    /// Tool name + version
    pub tool: Option<String>,
}

impl KonarrProject {
    /// Get or create a new snapshot for a project
    ///
    /// This will create a new snapshot ifthe following conditions are met:
    ///
    /// - Container SHA is different from the last snapshot
    /// - Tool + Version is different from the last snapshot
    /// - Last snapshot is older than 24 hours
    pub async fn snapshot(
        &mut self,
        client: &KonarrClient,
        data: &KonarrProjectSnapshotData,
    ) -> Result<KonarrSnapshot, KonarrError> {
        let version = data.info.clone().unwrap_or_default().version()?;

        if let Some(snap) = &self.snapshot {
            // Force a re-scan flag
            if let Some(data) = snap.metadata.get("rescan") {
                if data == "true" {
                    info!("Force rescan flag is set, creating new snapshot");
                    self.create_snapshot(client).await?;
                    return Ok(self.snapshot.clone().unwrap());
                }
            }

            // If the tool+version has changes (updated) we need to create a new snapshot
            if let Some(data_tool) = &data.tool {
                log::debug!("Local tool version: {}", data_tool);
                if let Some(tool) = snap.metadata.get("bom.tool") {
                    if version.major == 0 && version.minor <= 3 {
                        // +v0.3 = name@version
                        log::debug!("Remote Tool version: {}", tool);
                        if tool != data_tool {
                            info!("Tool version is different, creating new snapshot with new tool");
                            self.create_snapshot(client).await?;
                            return Ok(self.snapshot.clone().unwrap());
                        }
                    } else {
                        // For backwards compatibility
                        log::warn!(
                            "Checking tool name and version does not work for v0.2 or below"
                        );
                    }
                }
            }

            // Time passed
            let now = chrono::Utc::now();
            if now.signed_duration_since(snap.created_at).num_hours() >= 24 {
                info!("24 hours has passed since the last snapshot, creating a new one");
                self.create_snapshot(client).await?;
                return Ok(self.snapshot.clone().unwrap());
            }

            // Check container SHA
            if let Some(container_sha) = &data.container_sha {
                if let Some(sha) = snap.metadata.get("container.sha") {
                    debug!("Container Snapshot SHA: {} == {}", &container_sha, sha);
                    if sha != container_sha {
                        info!("Snapshot SHA for Container is different: {}", self.name);
                        self.create_snapshot(client).await?;
                        return Ok(self.snapshot.clone().unwrap());
                    }
                    log::debug!("Container SHAs are the same, skipping creation");
                } else {
                    debug!("Creating new Snapshot for Container: {}", self.name);
                    self.create_snapshot(client).await?;
                }
            }
        } else {
            info!("Creating initial Snapshot...");
            self.create_snapshot(client).await?;
        }

        Ok(self.snapshot.clone().unwrap())
    }

    /// Create a new snapshot for a project and update the snapshot field
    async fn create_snapshot(&mut self, client: &KonarrClient) -> Result<(), KonarrError> {
        let snap = KonarrSnapshot::create(client, self.id).await?;
        self.snapshot = Some(snap);
        Ok(())
    }
}
