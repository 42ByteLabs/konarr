//! # Snapshot Model

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use chrono::{DateTime, Utc};
use geekorm::{Connection, prelude::*};
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
    KonarrError,
    bom::{
        BillOfMaterials, BillOfMaterialsBuilder, BomParser, Parsers,
        cyclonedx::spec_v1_6::Bom as CycloneDx,
    },
    models::{
        Alerts, Dependencies, ProjectSnapshots, Projects, ServerSettings, Setting,
        security::SecuritySeverity,
    },
};

pub mod metadata;

pub use metadata::{SnapshotMetadata, SnapshotMetadataKey};

/// HashMap of Alerts Summary
pub type AlertsSummary = HashMap<SecuritySeverity, u16>;

/// Snapshot Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Snapshot State
    #[geekorm(new = "SnapshotState::Created")]
    pub state: SnapshotState,

    /// Datetime Created
    #[geekorm(new = "Utc::now()")]
    pub created_at: DateTime<Utc>,

    /// Last Updated / Checked for Changes
    #[serde(default)]
    #[geekorm(update = "Some(Utc::now())")]
    pub updated_at: Option<DateTime<Utc>>,

    /// SBOM (Bill of Materials) as Binary Data
    #[serde(default)]
    sbom: Option<Vec<u8>>,

    /// Error Message (if any)
    pub error: Option<String>,

    /// Components
    #[geekorm(skip)]
    #[serde(skip)]
    pub components: Vec<Dependencies>,

    /// Count of the Components
    #[geekorm(skip)]
    #[serde(skip)]
    pub components_count: usize,

    /// Snapshot Metadata
    #[geekorm(skip)]
    #[serde(skip)]
    pub metadata: HashMap<SnapshotMetadataKey, SnapshotMetadata>,

    /// Snapshot Alerts
    #[geekorm(skip)]
    #[serde(skip)]
    pub alerts: Vec<Alerts>,
}

impl Snapshot {
    /// Get all Snapshots
    pub async fn all(connection: &Connection<'_>) -> Result<Vec<Self>, crate::KonarrError> {
        Ok(Snapshot::query(
            connection,
            Snapshot::query_select()
                .order_by("created_at", QueryOrder::Asc)
                .build()?,
        )
        .await?)
    }

    /// Count snapshots dependencies
    pub async fn count_dependencies(
        &self,
        connection: &Connection<'_>,
    ) -> Result<usize, crate::KonarrError> {
        Ok(Dependencies::row_count(
            connection,
            Dependencies::query_count()
                .where_eq("snapshot_id", self.id)
                .build()?,
        )
        .await? as usize)
    }

    /// Fetch Project for the Snapshot
    pub async fn fetch_project(
        &self,
        connection: &Connection<'_>,
    ) -> Result<Projects, crate::KonarrError> {
        let snaps = ProjectSnapshots::fetch_by_snapshot_id(connection, self.id).await?;
        let snap = snaps.first().ok_or_else(|| geekorm::Error::NoRowsFound {
            query: format!("Cannot find first project snapshot: {}", self.id),
        })?;
        Ok(Projects::fetch_by_primary_key(connection, snap.project_id.clone()).await?)
        // TODO: Add JOIN
        // // SELECT * FROM Projects JOIN ProjectSnapshots ON Projects.id = ProjectSnapshots.project_id WHERE ProjectSnapshots.snapshot_id = 35
        // Ok(Projects::query_first(
        //     connection,
        //     Projects::query_select()
        //         .join(ProjectSnapshots::table())
        //         .where_eq("ProjectSnapshots.snapshot_id", self.id)
        //         .limit(1)
        //         .build()?,
        // )
        // .await?)
    }

    /// Find or create a new Snapshot from Bill of Materials
    ///
    /// If the snapshot already exists, it will return the existing snapshot.
    pub async fn from_bom(
        connection: &Connection<'_>,
        bom: &BillOfMaterials,
    ) -> Result<Self, crate::KonarrError> {
        let connection = connection.into();
        // Based on the SHA, check if the snapshot already exists
        let mut snapshot: Snapshot =
            match SnapshotMetadata::find_by_sha(connection, bom.sha.clone()).await {
                Ok(Some(meta)) => {
                    debug!("Snapshot Found with same SHA :: {:?}", meta);
                    let mut snap =
                        Snapshot::fetch_by_primary_key(connection, meta.snapshot_id).await?;
                    snap.fetch(connection).await?;
                    snap.fetch_metadata(connection).await?;

                    snap
                }
                _ => {
                    let mut snap = Self::new();
                    snap.save(connection).await?;
                    snap
                }
            };

        // Inline processing of the BOM
        snapshot.process_bom(connection, bom).await?;

        Ok(snapshot)
    }

    /// Add Bill of Materials to the Snapshot
    pub async fn add_bom(
        &mut self,
        connection: &Connection<'_>,
        bom: Vec<u8>,
    ) -> Result<(), crate::KonarrError> {
        self.state = SnapshotState::Created;
        self.sbom = Some(bom);
        self.update(connection).await?;
        Ok(())
    }

    /// Process the Bill of Materials to create Dependencies
    pub async fn process_bom(
        &mut self,
        connection: &Connection<'_>,
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
            SnapshotMetadata::update_or_create(connection, self.id, &key, value).await?;
        }
        // Tools
        // TODO: Supporting multiple tools (for now, only one tool)
        for tool in bom.tools.iter() {
            SnapshotMetadata::update_or_create(
                connection,
                self.id,
                &SnapshotMetadataKey::BomToolName,
                tool.name.clone(),
            )
            .await?;
            if !tool.version.is_empty() {
                SnapshotMetadata::update_or_create(
                    connection,
                    self.id,
                    &SnapshotMetadataKey::BomToolVersion,
                    tool.version.clone(),
                )
                .await?;
            }

            let name = format!("{}@{}", tool.name, tool.version);
            SnapshotMetadata::update_or_create(
                connection,
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
                connection,
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
                connection,
                self.id,
                &SnapshotMetadataKey::ContainerVersion,
                version.clone(),
            )
            .await?;
        }

        for comp in bom.components.iter() {
            // Create dependency from PURL
            Dependencies::from_bom_compontent(connection, self.id, comp).await?;
        }
        info!("Finished indexing dependencies");

        if ServerSettings::feature_security(connection).await? {
            info!("Indexing Security Alerts from BillOfMaterials");

            for vuln in bom.vulnerabilities.iter() {
                Alerts::from_bom_vulnerability(connection, self, vuln).await?;
            }
            SnapshotMetadata::update_or_create(
                connection,
                self.id,
                &SnapshotMetadataKey::SecurityToolsAlerts,
                "true",
            )
            .await?;

            // Calculate the totals
            info!("Calculating Security Alert Totals");
            self.calculate_alerts_summary(connection).await?;
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
                let page = Page::from((0, deps_count as u32));
                self.components = self.fetch_dependencies(connection, &page).await?;
            }

            let bom_file_path = if let Some(path) = self.find_metadata("bom.path") {
                path.clone()
            } else if !self.components.is_empty() {
                log::debug!("SBOM metadata not found, but components found");
                log::info!(
                    "Building SBOM from components: {} - {}",
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
                log::error!("Failed to parse SBOM");
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

    /// Set the state of the Snapshot and add an error message
    pub async fn set_error(
        &mut self,
        connection: &Connection<'_>,
        error: String,
    ) -> Result<(), crate::KonarrError> {
        self.state = SnapshotState::Failed;
        self.error = Some(error);
        self.updated_at = Some(Utc::now());
        self.update(connection).await?;
        Ok(())
    }

    /// Reset error and state of the Snapshot
    pub async fn reset_error(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<(), crate::KonarrError> {
        self.state = SnapshotState::Created;
        self.error = None;
        self.updated_at = Some(Utc::now());
        self.update(connection).await?;
        Ok(())
    }

    /// Rescan the Project
    pub async fn rescan(&mut self, connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        self.set_metadata(connection, SnapshotMetadataKey::Rescan, "true")
            .await?;
        Ok(())
    }

    /// Fetch Dependencies for the Snapshot
    pub async fn fetch_dependencies(
        &self,
        connection: &Connection<'_>,
        page: &Page,
    ) -> Result<Vec<Dependencies>, crate::KonarrError> {
        Dependencies::query(
            connection,
            Dependencies::query_select()
                .where_eq("snapshot_id", self.id)
                .page(page)
                .build()?,
        )
        .await
        .map_err(|e| e.into())
    }

    /// Find Metadata by Key
    pub fn find_metadata(&self, key: &str) -> Option<&SnapshotMetadata> {
        let key = SnapshotMetadataKey::from_str(key).ok()?;
        self.metadata.get(&key)
    }
    /// Find Metadata by Key and return as usize
    pub fn find_metadata_usize(&self, key: &str) -> usize {
        self.find_metadata(key).map_or(0, |m| m.as_i32() as usize)
    }

    /// Set Metadata for the Snapshot
    pub async fn set_metadata(
        &mut self,
        connection: &Connection<'_>,
        key: impl Into<SnapshotMetadataKey>,
        value: &str,
    ) -> Result<(), crate::KonarrError> {
        let key = key.into();
        SnapshotMetadata::update_or_create(connection, self.id, &key, value).await?;
        Ok(())
    }

    /// Fetch Snapshot by ID
    pub async fn fetch_metadata(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<(), crate::KonarrError> {
        let metadata = SnapshotMetadata::query(
            connection,
            SnapshotMetadata::query_select()
                .where_eq("snapshot_id", self.id)
                .build()?,
        )
        .await?;

        self.metadata = metadata.into_iter().map(|m| (m.key.clone(), m)).collect();

        Ok(())
    }

    /// Fetch Alerts for the Snapshot
    pub async fn fetch_alerts(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<&Vec<Alerts>, crate::KonarrError> {
        let mut alerts = Alerts::fetch_by_snapshot_id(connection, self.id).await?;
        for alert in alerts.iter_mut() {
            alert.fetch_advisory_id(connection).await?;
        }

        log::debug!("Found {} Alerts for Snapshot({})", alerts.len(), self.id);
        self.alerts = alerts;

        Ok(&self.alerts)
    }

    /// Count the number of Alerts for the Snapshot
    pub async fn fetch_alerts_count(
        &self,
        connection: &Connection<'_>,
    ) -> Result<usize, crate::KonarrError> {
        Ok(Alerts::row_count(
            connection,
            Alerts::query_count()
                .where_eq("snapshot_id", self.id)
                .build()?,
        )
        .await? as usize)
    }

    /// Fetch Alerts for the Snapshot with Pagination
    pub async fn fetch_alerts_page(
        &self,
        connection: &Connection<'_>,
        page: &Page,
    ) -> Result<Vec<Alerts>, crate::KonarrError> {
        let mut alerts = Alerts::query(
            connection,
            Alerts::query_select()
                .where_eq("snapshot_id", self.id)
                .page(page)
                .build()?,
        )
        .await?;

        for alert in alerts.iter_mut() {
            alert.fetch(connection).await?;
        }
        Ok(alerts)
    }

    /// Calculate a Summary of the Alerts and store in Metadata
    pub async fn calculate_alerts_summary(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<AlertsSummary, KonarrError> {
        let mut summary: HashMap<SecuritySeverity, u16> = HashMap::new();

        let mut alerts = Alerts::fetch_by_snapshot_id(connection, self.id).await?;
        log::debug!("Calculating Alert Summary for {} Alerts", alerts.len());

        for alert in alerts.iter_mut() {
            let advisory = alert.fetch_advisory_id(connection).await?;
            let severity = advisory.severity.clone();

            *summary.entry(severity).or_insert(0) += 1;
        }

        self.calculate_alerts(connection, &summary).await?;
        Ok(summary)
    }

    /// Calculate the Alert Totals
    pub async fn calculate_alerts(
        &mut self,
        connection: &Connection<'_>,
        summary: &HashMap<SecuritySeverity, u16>,
    ) -> Result<(), KonarrError> {
        debug!("Calculating Alert Totals for Snapshot({})", self.id);

        let mut total = 0;
        for (severity, count) in summary {
            self.set_metadata(
                connection,
                &format!("security.alerts.{}", severity.to_string().to_lowercase()),
                &count.to_string(),
            )
            .await?;
            total += count;
        }

        self.set_metadata(
            connection,
            SnapshotMetadataKey::SecurityAlertTotal,
            total.to_string().as_str(),
        )
        .await?;
        log::debug!("Alert Summary for Snapshot({}): {:?}", self.id, total);

        Ok(())
    }
}

/// Snapshot State
#[derive(Data, Debug, Clone, Default, PartialEq, Eq)]
pub enum SnapshotState {
    /// Snapshot Created (but not processed)
    #[default]
    Created,
    /// Snapshot Processing (in progress)
    Processing,
    /// Snapshot Completed (finished and ready for use)
    Completed,
    /// Snapshot Failed (error during processing)
    Failed,
}
