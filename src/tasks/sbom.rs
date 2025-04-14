//! SBOM Task
use geekorm::{ConnectionManager, GeekConnector};

use crate::models::Snapshot;
use crate::models::dependencies::snapshots::SnapshotState;

use super::TaskTrait;

/// SbomTask is for processing Snapshot SBOMs and creating Dependency Trees
#[derive(Default)]
pub struct SbomTask;

#[async_trait::async_trait]
impl TaskTrait for SbomTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let mut snapshots =
            Snapshot::fetch_by_state(&database.acquire().await, SnapshotState::Created).await?;
        log::debug!("Processing {} Snapshots", snapshots.len());

        for snapshot in snapshots.iter_mut() {
            log::info!("Processing Snapshot: {:?}", snapshot);
            snapshot.state = SnapshotState::Processing;
            snapshot.update(&database.acquire().await).await?;

            log::debug!("Fetching SBOM for Snapshot: {:?}", snapshot);
            let bom = snapshot.get_bom(&database.acquire().await).await?;
            log::debug!("Parsed SBOM: {:?}", bom);

            if let Err(err) = snapshot.process_bom(&database.acquire().await, &bom).await {
                log::error!("Failed to process SBOM: {:?}", err);
                snapshot
                    .set_error(&database.acquire().await, err.to_string())
                    .await?;
            };

            log::debug!("Processing SBOM for Snapshot: {:?}", snapshot);
            snapshot.state = SnapshotState::Completed;
            snapshot.update(&database.acquire().await).await?;
        }

        Ok(())
    }
}
