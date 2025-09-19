//! SBOM Task
use geekorm::ConnectionManager;

use crate::models::Snapshot;
use crate::models::dependencies::snapshots::SnapshotState;

use super::TaskTrait;

/// SbomTask is for processing Snapshot SBOMs and creating Dependency Trees
#[derive(Default)]
pub struct SbomTask {
    state: SnapshotState,
    id: Option<i32>,
}

#[async_trait::async_trait]
impl TaskTrait for SbomTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let mut snapshots = if let Some(id) = self.id {
            log::info!("Processing Snapshot ID: {}", id);
            Snapshot::fetch_by_id(&database.acquire().await, id).await?
        } else {
            Snapshot::fetch_by_state(&database.acquire().await, &self.state).await?
        };

        log::debug!("Processing {} Snapshots", snapshots.len());

        for snapshot in snapshots.iter_mut() {
            log::debug!("Processing Snapshot: {:?}", snapshot);
            snapshot
                .set_state(&database.acquire().await, SnapshotState::Processing)
                .await?;

            log::debug!("Fetching SBOM for Snapshot: {:?}", snapshot);
            let bom = if let Ok(bom) = snapshot.get_bom(&database.acquire().await).await {
                bom
            } else {
                snapshot
                    .set_error(&database.acquire().await, "Failed to fetch/load SBOM")
                    .await?;
                continue;
            };
            log::debug!("Parsed SBOM: {:?}", bom);

            if let Err(err) = snapshot.process_bom(&database.acquire().await, &bom).await {
                snapshot
                    .set_error(&database.acquire().await, err.to_string())
                    .await?;
            } else {
                snapshot
                    .set_state(&database.acquire().await, SnapshotState::Completed)
                    .await?;
            }
        }

        Ok(())
    }
}

impl SbomTask {
    /// Create a new SbomTask for a specific snapshot
    pub fn sbom(id: i32) -> Self {
        Self {
            id: Some(id),
            ..Default::default()
        }
    }

    /// Create a new SbomTask for a specific state
    pub fn sbom_by_state(state: impl Into<SnapshotState>) -> Self {
        Self {
            state: state.into(),
            ..Default::default()
        }
    }

    /// Scan the failed snapshots
    pub fn failed() -> Self {
        Self {
            state: SnapshotState::Failed,
            ..Default::default()
        }
    }
}
