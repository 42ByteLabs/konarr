//! # Snapshot Bill of Materials
use geekorm::{Connection, ConnectionManager, GeekConnector};
use log::{debug, info};
use std::path::PathBuf;

use super::{Snapshot, SnapshotMetadata, SnapshotState};
use crate::{
    KonarrError,
    bom::{
        BillOfMaterials, BillOfMaterialsBuilder, BomParser, Parsers,
        cyclonedx::spec_v1_6::Bom as CycloneDx,
    },
    models::{Alerts, Dependencies, ServerSettings, Setting, SnapshotMetadataKey},
    tasks::{CatalogueTask, TaskTrait},
};

const SBOM_MIN_SIZE: usize = 100;

impl Snapshot {
    /// Check if the Snapshot has an SBOM
    pub fn has_sbom(&self) -> bool {
        self.sbom.is_some()
    }

    /// Find or create a new Snapshot from Bill of Materials
    ///
    /// If the snapshot already exists, it will return the existing snapshot.
    pub async fn from_bom(
        database: &ConnectionManager,
        bom: &BillOfMaterials,
    ) -> Result<Self, crate::KonarrError> {
        // Based on the SHA, check if the snapshot already exists
        let mut snapshot: Snapshot =
            match SnapshotMetadata::find_by_sha(&database.acquire().await, bom.sha.clone()).await {
                Ok(Some(meta)) => {
                    debug!("Snapshot Found with same SHA :: {:?}", meta);
                    let mut snap =
                        Snapshot::fetch_by_primary_key(&database.acquire().await, meta.snapshot_id)
                            .await?;
                    snap.fetch(&database.acquire().await).await?;
                    snap.fetch_metadata(&database.acquire().await).await?;

                    snap
                }
                _ => {
                    let mut snap = Self::new();
                    snap.save(&database.acquire().await).await?;
                    snap
                }
            };

        // Inline processing of the BOM
        snapshot.process_bom(&database, bom).await?;

        Ok(snapshot)
    }

    /// Add Bill of Materials to the Snapshot
    pub async fn add_bom(
        &mut self,
        connection: &Connection<'_>,
        bom: Vec<u8>,
    ) -> Result<(), crate::KonarrError> {
        // Make sure we aren't uploading small files that can't possibly be a file
        if bom.len() < SBOM_MIN_SIZE {
            self.set_error(connection, "SBOM file is too small").await?;
            return Err(KonarrError::ParseSBOM("SBOM file is too small".to_string()));
        }

        self.state = SnapshotState::Created;
        debug!("Updating snapshot state to `Created`");
        self.sbom = Some(bom);
        debug!("SBOM({}) is {} bytes", self.id, self.sbom.iter().len());
        self.update(connection).await?;
        Ok(())
    }

    /// Process the Bill of Materials to create Dependencies
    pub async fn process_bom(
        &mut self,
        database: &ConnectionManager,
        bom: &BillOfMaterials,
    ) -> Result<(), crate::KonarrError> {
        let metadata = vec![
            (SnapshotMetadataKey::BomType, bom.sbom_type.to_string()),
            (SnapshotMetadataKey::BomVersion, bom.version.clone()),
            (
                SnapshotMetadataKey::DependenciesTotal,
                bom.components.len().to_string(),
            ),
            (SnapshotMetadataKey::BomSha, bom.sha.clone()),
        ];
        for (key, value) in metadata {
            SnapshotMetadata::update_or_create(&database.acquire().await, self.id, &key, value)
                .await?;
        }
        // Tools
        // TODO: Supporting multiple tools (for now, only one tool)
        for tool in bom.tools.iter() {
            SnapshotMetadata::update_or_create(
                &database.acquire().await,
                self.id,
                &SnapshotMetadataKey::BomToolName,
                tool.name.clone(),
            )
            .await?;
            if !tool.version.is_empty() {
                SnapshotMetadata::update_or_create(
                    &database.acquire().await,
                    self.id,
                    &SnapshotMetadataKey::BomToolVersion,
                    tool.version.clone(),
                )
                .await?;
            }

            let name = format!("{}@{}", tool.name, tool.version);
            SnapshotMetadata::update_or_create(
                &database.acquire().await,
                self.id,
                &SnapshotMetadataKey::BomTool,
                name,
            )
            .await?;
        }

        // Container Metadata
        if let Some(image) = &bom.container.image {
            // TODO: Assume its from docker.io by default? Latest?
            SnapshotMetadata::update_or_create(
                &database.acquire().await,
                self.id,
                &SnapshotMetadataKey::ContainerImage,
                image.clone(),
            )
            .await?;
            // TODO: Parse the image to get the registry, repository, tag
        }
        // TODO: Assume latest?
        if let Some(version) = &bom.container.version {
            SnapshotMetadata::update_or_create(
                &database.acquire().await,
                self.id,
                &SnapshotMetadataKey::ContainerVersion,
                version.clone(),
            )
            .await?;
        }

        for comp in bom.components.iter() {
            // Create dependency from PURL
            Dependencies::from_bom_compontent(&database.acquire().await, self.id, comp).await?;
        }
        info!("Finished indexing dependencies");

        CatalogueTask::snapshot(self.id)
            .spawn_task(&database)
            .await?;

        if ServerSettings::feature_security(&database.acquire().await).await? {
            info!("Indexing Security Alerts from BillOfMaterials");

            for vuln in bom.vulnerabilities.iter() {
                Alerts::from_bom_vulnerability(&database.acquire().await, self, vuln).await?;
            }
            SnapshotMetadata::update_or_create(
                &database.acquire().await,
                self.id,
                &SnapshotMetadataKey::SecurityToolsAlerts,
                "true",
            )
            .await?;

            // Calculate the totals
            info!("Calculating Security Alert Totals");
            self.calculate_alerts_summary(&database.acquire().await)
                .await?;
        }

        Ok(())
    }

    /// Gets the SBOM from the database (v0.5+) or disk (v0.3 -> v0.4)
    ///
    /// If the SBOM is not found in the database, it will try to read it from disk.
    /// If the SBOM is found on disk, it will be added to the database.
    pub async fn sbom(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<Vec<u8>, crate::KonarrError> {
        // v0.5+ stores the SBOM in the database
        if let Some(bomdata) = &self.sbom {
            log::debug!("SBOM found in database");
            Ok(bomdata.clone())
        } else {
            // v0.4 only stores the SBOM on disk
            log::debug!("SBOM not found in database, trying to read from disk");

            let data_path = ServerSettings::fetch_by_name(connection, Setting::ServerData).await?;
            let sbom_path = PathBuf::from(data_path.value.clone()).join("sboms");

            // We might have the dependency data in the database but no SBOM
            let deps_count = self.count_dependencies(connection).await?;
            if deps_count != 0 {
                log::debug!("Found {} dependencies", deps_count);
                self.components = self.fetch_all_dependencies(connection).await?;
            }

            let bom_file_path = if let Some(path) = self.find_metadata("bom.path") {
                path.clone()
            } else if !self.components.is_empty() {
                log::debug!("SBOM metadata not found, but components found");
                log::info!(
                    "Building SBOM from components: Id({}) - Comps({})",
                    self.id,
                    self.components.len()
                );
                // Build a new SBOM from the components
                let mut bom = CycloneDx::new();
                bom.add_project(&self.fetch_project(connection).await?)?;
                bom.add_dependencies(&self.components)?;

                self.rescan(connection).await?;

                return bom.output();
            } else {
                // If not found, we don't know how to get the SBOM
                log::error!("SBOM file path not found in metadata");
                self.rescan(connection).await?;
                self.set_error(connection, "Unable to find SBOM file".to_string())
                    .await?;
                return Err(KonarrError::SBOMNotFound(
                    "Unable to find SBOM file".to_string(),
                ));
            };

            let bom_path = sbom_path.join(bom_file_path.as_string());

            let sbom_data = if bom_path.exists() {
                log::debug!("SBOM file found: {}", bom_path.display());
                // Read the SBOM from disk
                tokio::fs::read(&bom_path).await?
            } else if !self.components.is_empty() {
                log::debug!("SBOM file not found, but components found");
                log::debug!("Building SBOM from components");
                // Build a new SBOM from the components
                let mut bom = CycloneDx::new();
                bom.add_project(&self.fetch_project(connection).await?)?;
                bom.add_dependencies(&self.components)?;
                self.rescan(connection).await?;

                return bom.output();
            } else {
                log::error!("SBOM file not found: {}", bom_path.display());

                self.rescan(connection).await?;
                self.set_error(connection, "Unable to find SBOM file".to_string())
                    .await?;
                return Err(KonarrError::SBOMNotFound(bom_path.display().to_string()));
            };

            self.add_bom(connection, sbom_data.clone()).await?;

            log::debug!("Deleting old SBOM file and metadata");

            bom_file_path.delete(connection).await?;
            if bom_path.exists() {
                log::debug!("Deleting SBOM file: {}", bom_path.display());
                std::fs::remove_file(bom_path)?;
            }

            Ok(sbom_data)
        }
    }

    /// Get the Bill of Materials (SBOM) from the database
    pub async fn get_bom(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<BillOfMaterials, crate::KonarrError> {
        let sbom = self.sbom(connection).await?;
        match Parsers::parse(&sbom) {
            Ok(bom) => {
                log::debug!("Parsed SBOM: {:?}", bom);
                Ok(bom)
            }
            Err(err) => {
                log::error!("Failed to parse SBOM, requesting rescan and setting error");
                self.rescan(connection).await?;
                self.set_error(connection, "Failed to parse SBOM".to_string())
                    .await?;
                Err(KonarrError::ParseSBOM(format!(
                    "Failed to parse SBOM: {}",
                    err
                )))
            }
        }
    }
}
