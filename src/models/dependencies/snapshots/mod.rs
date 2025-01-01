//! # Snapshot Model

use std::{collections::HashMap, str::FromStr};

use chrono::{DateTime, Utc};
use geekorm::prelude::*;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
    bom::BillOfMaterials,
    models::{
        security::{SecuritySeverity, SecurityState},
        Alerts, Dependencies, ServerSettings,
    },
    KonarrError,
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

    /// Components
    #[geekorm(skip)]
    #[serde(skip)]
    pub components: Vec<Dependencies>,

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
    /// Create a new Snapshot
    pub async fn create<'a, T>(connection: &'a T) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut snapshot = Snapshot::new();
        snapshot.save(connection).await?;
        debug!("Creating Snapshot: {:?}", snapshot);
        Ok(snapshot)
    }

    /// Get all Snapshots
    pub async fn all<'a, T>(connection: &'a T) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Snapshot::query(
            connection,
            Snapshot::query_select()
                .order_by("created_at", QueryOrder::Asc)
                .build()?,
        )
        .await?)
    }

    /// Find or create a new Snapshot from Bill of Materials
    ///
    /// If the snapshot already exists, it will return the existing snapshot.
    pub async fn from_bom<'a, T>(
        connection: &'a T,
        bom: &BillOfMaterials,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
                _ => Self::create(connection).await?,
            };

        snapshot.add_bom(connection, bom).await?;

        Ok(snapshot)
    }

    /// Add Bill of Materials to the Snapshot
    pub async fn add_bom<'a, T>(
        &mut self,
        connection: &'a T,
        bom: &BillOfMaterials,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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

    /// Fetch Dependencies for the Snapshot
    pub async fn fetch_dependencies<'a, T>(
        &self,
        connection: &'a T,
        page: usize,
        limit: usize,
    ) -> Result<Vec<Dependencies>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Dependencies::query(
            connection,
            Dependencies::query_select()
                .where_eq("snapshot_id", self.id)
                .limit(limit)
                .offset(page * limit)
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
    pub async fn set_metadata<'a, T>(
        &mut self,
        connection: &'a T,
        key: impl Into<SnapshotMetadataKey>,
        value: &str,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let key = key.into();
        SnapshotMetadata::update_or_create(connection, self.id, &key, value).await?;
        Ok(())
    }

    /// Fetch Snapshot by ID
    pub async fn fetch_metadata<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
    pub async fn fetch_alerts<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<&Vec<Alerts>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut alerts = Alerts::fetch_by_snapshot_id(connection, self.id).await?;
        for alert in alerts.iter_mut() {
            alert.fetch_advisory_id(connection).await?;
        }

        self.alerts = alerts;

        Ok(&self.alerts)
    }

    /// Count the number of Alerts for the Snapshot
    pub async fn fetch_alerts_count<'a, T>(
        &self,
        connection: &'a T,
    ) -> Result<usize, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Alerts::row_count(
            connection,
            Alerts::query_count()
                .where_eq("snapshot_id", self.id)
                .build()?,
        )
        .await? as usize)
    }

    /// Fetch Alerts for the Snapshot with Pagination
    pub async fn fetch_alerts_page<'a, T>(
        &self,
        connection: &'a T,
        page: &Pagination,
    ) -> Result<Vec<Alerts>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
    pub async fn calculate_alerts_summary<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<AlertsSummary, KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut summary: HashMap<SecuritySeverity, u16> = HashMap::new();

        let mut alerts = Alerts::fetch_by_snapshot_id(connection, self.id).await?;
        log::debug!("Calculating Alert Summary for {} Alerts", alerts.len());

        for alert in alerts.iter_mut() {
            if alert.state != SecurityState::Vulnerable {
                continue;
            }
            let advisory = alert.fetch_advisory_id(connection).await?;
            let severity = advisory.severity.clone();

            *summary.entry(severity).or_insert(0) += 1;
        }

        self.calculate_alerts(connection, &summary).await?;
        Ok(summary)
    }

    /// Calculate the Alert Totals
    pub async fn calculate_alerts<'a, T>(
        &mut self,
        connection: &'a T,
        summary: &HashMap<SecuritySeverity, u16>,
    ) -> Result<(), KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
        debug!("Alert Summary for Snapshot({}): {:?}", self.id, total);
        self.set_metadata(connection, "security.alerts.total", &total.to_string())
            .await?;

        Ok(())
    }
}

/// Snapshot State
#[derive(Data, Debug, Clone, Default)]
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
