//! # Snapshot Model

use chrono::{DateTime, Utc};
use geekorm::{Connection, prelude::*};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use crate::{
    KonarrError,
    models::{Alerts, Dependencies, ProjectSnapshots, Projects, security::SecuritySeverity},
};

pub mod metadata;
pub mod sboms;

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

    /// Fetch all Dependencies for the Snapshot
    pub async fn fetch_all_dependencies(
        &self,
        connection: &Connection<'_>,
    ) -> Result<Vec<Dependencies>, crate::KonarrError> {
        let mut deps = Dependencies::query(
            connection,
            Dependencies::query_select()
                .where_eq("snapshot_id", self.id)
                .build()?,
        )
        .await?;

        for dep in deps.iter_mut() {
            dep.fetch(connection).await?;
        }
        Ok(deps)
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
