//! # Task - Catalogue
use geekorm::{ConnectionManager, prelude::*};

use super::TaskTrait;
use crate::{
    models::{Component, ComponentType, SnapshotMetadataKey, SnapshotState},
    utils::catalogue::Catalogue,
};

/// Catalogue the components task
#[derive(Default)]
pub struct CatalogueTask {
    force: bool,
    snapshot: Option<i32>,
}

#[async_trait::async_trait]
impl TaskTrait for CatalogueTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        log::info!("Starting Catalogue Task");
        let catalogue = Catalogue::new();

        let mut counter = 0;
        let mut comps = Component::all(&database.acquire().await).await?;
        log::debug!("Checking component types for `{}` Components", comps.len());

        for mut comp in comps.iter_mut() {
            if !self.force {
                match comp.component_type {
                    ComponentType::Unknown
                    | ComponentType::Library
                    | ComponentType::Application => {
                        if catalogue.catalogue(&mut comp)? {
                            log::info!("Updating component_type: {}", comp.component_type);
                            comp.update(&database.acquire().await).await?;
                            counter += 1;
                        }
                    }
                    _ => {}
                }
            } else if self.force && catalogue.catalogue(&mut comp)? {
                log::info!("Updating component_type: {}", comp.component_type);
                comp.update(&database.acquire().await).await?;
                counter += 1;
            }
        }
        if counter != 0 {
            log::info!("Updated `{}` component out of `{}`", counter, comps.len());
        }

        if self.snapshot.is_some() {
            self.process_snapshot(database).await?;
        } else {
            for snapshot in crate::models::Snapshot::fetch_by_state(
                &database.acquire().await,
                SnapshotState::Completed,
            )
            .await?
            {
                log::info!("Processing Snapshot ID: {}", snapshot.id);
                self.process_snapshot(database).await?;
            }
        }

        Ok(())
    }
}

impl CatalogueTask {
    /// Set the Catalogue Task to force
    pub fn force() -> Self {
        Self {
            force: true,
            snapshot: None,
        }
    }
    /// Set the Catalogue Task to a specific snapshot
    pub fn snapshot(id: impl Into<i32>) -> Self {
        Self {
            force: false,
            snapshot: Some(id.into()),
        }
    }

    /// Once the catalogue has been run, process the snapshot metadata.
    /// This function will process a specific snapshot and catalogue its components
    /// and update the snapshot metadata accordingly.
    pub async fn process_snapshot(
        &self,
        database: &ConnectionManager,
    ) -> Result<(), crate::KonarrError> {
        if let Some(snap_id) = self.snapshot {
            log::info!("Processing catalogue Snapshot ID: {}", snap_id);
            let mut snapshot =
                crate::models::Snapshot::fetch_by_primary_key(&database.acquire().await, snap_id)
                    .await?;
            let dependencies = snapshot
                .fetch_all_dependencies(&database.acquire().await)
                .await?;

            for dependency in dependencies {
                let comp = dependency.component();
                match comp.component_type {
                    ComponentType::OperatingSystem => {
                        // Capture OS Name and Version
                        snapshot
                            .set_metadata(
                                &database.acquire().await,
                                SnapshotMetadataKey::Os,
                                &dependency.name(),
                            )
                            .await?;

                        if let Some(version) = dependency.version() {
                            snapshot
                                .set_metadata(
                                    &database.acquire().await,
                                    SnapshotMetadataKey::OsVersion,
                                    &version,
                                )
                                .await?;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            log::warn!("No Snapshot ID provided for processing");
        }
        Ok(())
    }
}
