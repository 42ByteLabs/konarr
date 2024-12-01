//! # Snapshot Model

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use geekorm::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};

#[cfg(feature = "tools-grypedb")]
use crate::utils::grypedb::GrypeVulnerability;
use crate::{
    bom::BillOfMaterials,
    models::{security::SecuritySeverity, Alerts, Dependencies, ServerSettings},
    KonarrError,
};

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
    pub metadata: HashMap<String, SnapshotMetadata>,

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
        let metadata: Vec<(&str, String)> = vec![
            ("bom.type", bom.sbom_type.to_string()),
            ("bom.version", bom.version.clone()),
            ("bom.dependencies.count", bom.components.len().to_string()),
            ("bom.sha", bom.sha.clone()),
        ];
        for (key, value) in metadata {
            SnapshotMetadata::update_or_create(connection, self.id, key, value).await?;
        }
        // Tools
        // TODO: Supporting multiple tools (for now, only one tool)
        for tool in bom.tools.iter() {
            SnapshotMetadata::update_or_create(connection, self.id, "bom.tool", tool.name.clone())
                .await?;
            if !tool.version.is_empty() {
                SnapshotMetadata::update_or_create(
                    connection,
                    self.id,
                    "bom.tool.version",
                    tool.version.clone(),
                )
                .await?;
            }
        }

        // Container Metadata
        if let Some(image) = &bom.container.image {
            // TODO: Assume its from docker.io by default? Latest?
            SnapshotMetadata::update_or_create(
                connection,
                self.id,
                "container.image",
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
                "container.version",
                version.clone(),
            )
            .await?;
        }

        for comp in bom.components.iter() {
            // Create dependency from PURL
            Dependencies::from_bom_compontent(connection, self.id, comp).await?;
        }

        if ServerSettings::feature_security(connection).await? {
            debug!("Indexing Security Alerts from BillOfMaterials");

            for vuln in bom.vulnerabilities.iter() {
                Alerts::from_bom_vulnerability(connection, self, vuln).await?;
            }
            SnapshotMetadata::update_or_create(
                connection,
                self.id,
                "security.tools.alerts",
                "true",
            )
            .await?;

            // Calculate the totals
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
        self.metadata.get(key)
    }
    /// Find Metadata by Key and return as usize
    pub fn find_metadata_usize(&self, key: &str) -> usize {
        self.find_metadata(key).map_or(0, |m| m.as_i32() as usize)
    }

    /// Set Metadata for the Snapshot
    pub async fn set_metadata<'a, T>(
        &mut self,
        connection: &'a T,
        key: &str,
        value: &str,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        SnapshotMetadata::update_or_create(connection, self.id, key, value).await?;
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

    /// Fetch Grype Results for the Snapshot
    #[cfg(feature = "tools-grypedb")]
    pub async fn scan_with_grype<'a, T>(
        &mut self,
        connection: &'a T,
        grypedb_connection: &'a T,
    ) -> Result<Vec<Alerts>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        use crate::{
            models::security::{Advisories, AdvisorySource, SecuritySeverity},
            utils::grypedb::GrypeVulnerabilityMetadata,
        };

        let mut results = Vec::new();

        if self.components.is_empty() {
            debug!("Fetching components as none are present");

            self.components = Dependencies::fetch_by_snapshot_id(connection, self.id).await?;
            for comp in self.components.iter_mut() {
                comp.fetch(connection).await?;
            }
        }
        debug!("Dependencies Count: {}", self.components.len());

        // Summary of the Security Alerts (cached)
        let mut summary: AlertsSummary = HashMap::new();

        for dependency in self.components.iter_mut() {
            log::debug!(
                "Scanning Dependency: {}",
                dependency.component_id.data.purl()
            );
            let vulns = GrypeVulnerability::find_vulnerabilities(
                grypedb_connection,
                &dependency.component_id.data,
                &dependency.component_version_id.data,
            )
            .await?;
            log::debug!(
                "Grype Results for {}@{} :: {}",
                dependency.component_id.data.purl(),
                dependency.component_version_id.data.version,
                vulns.len()
            );

            for vuln in &vulns {
                let vuln_metadata =
                    GrypeVulnerabilityMetadata::fetch_by_id(grypedb_connection, vuln.id.clone())
                        .await?;

                let severity = SecuritySeverity::from(vuln_metadata.severity.clone());

                // Advisory
                let mut advisory =
                    Advisories::new(vuln.id.clone(), AdvisorySource::Anchore, severity.clone());
                advisory.fetch_or_create(connection).await?;
                advisory.fetch_metadata(connection).await?;

                // Description
                if advisory
                    .get_metadata(connection, "description")
                    .await?
                    .is_none()
                {
                    if !vuln_metadata.description.is_empty() {
                        advisory
                            .add_metadata(
                                connection,
                                "description",
                                vuln_metadata.description.clone(),
                            )
                            .await?;
                    }
                }
                if let Some(cvss) = vuln_metadata.cvss {
                    advisory
                        .add_metadata(connection, "cvss", cvss.to_string())
                        .await?;
                }
                if let Some(link) = vuln_metadata.urls {
                    advisory.add_metadata(connection, "urls", link).await?;
                }
                advisory
                    .add_metadata(connection, "data.source", "GrypeDB".to_string())
                    .await?;

                let mut alert = Alerts::new(vuln.id.clone(), self.id, dependency.id, advisory.id);
                alert.find_or_create(connection).await?;

                *summary.entry(severity).or_insert(0) += 1;

                results.push(alert);
            }
        }

        self.calculate_alerts(connection, &summary).await?;

        Ok(results)
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
        for alert in alerts.iter_mut() {
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

/// Snapshot Metadata Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Snapshot ID
    #[geekorm(foreign_key = "Snapshot.id")]
    pub snapshot_id: ForeignKey<i32, Snapshot>,

    /// Key
    pub key: String,
    /// Value (any binary data)
    pub value: Vec<u8>,

    /// Datetime Created
    #[geekorm(new = "Utc::now()")]
    pub created_at: DateTime<Utc>,
    /// Last Updated
    #[geekorm(new = "Utc::now()")]
    pub updated_at: DateTime<Utc>,
}

impl SnapshotMetadata {
    /// Initialise SnapshotMetadata
    pub async fn init<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Self::create_table(connection).await?;

        Ok(())
    }

    /// Update or Create Metadata
    pub async fn update_or_create<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: impl Into<String>,
        value: impl Into<Vec<u8>>,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
        // TODO: Do we need to validate the key? This is user controlled
        let key = key.into();
        let value = value.into();
        debug!("Updating Metadata for Snapshot({:?}) :: {} ", snapshot, key);

        Ok(
            match Self::find_by_key(connection, snapshot, key.clone()).await {
                Ok(Some(mut meta)) => {
                    meta.value = value;
                    meta.updated_at = chrono::Utc::now();

                    meta.update(connection).await?;
                    meta
                }
                _ => Self::add(connection, snapshot, key, value).await?,
            },
        )
    }
    /// Add new Metadata to the Snapshot
    pub async fn add<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: impl Into<String>,
        value: impl Into<Vec<u8>>,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
        let key = key.into();
        debug!("Adding Metadata to Snapshot({:?}) :: {} ", snapshot, key);

        let mut meta = Self::new(snapshot, key, value.into());
        meta.save(connection).await?;
        Ok(meta)
    }

    /// Find Metadata by Key for a Snapshot
    pub async fn find_by_key<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: impl Into<String>,
    ) -> Result<Option<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
        let key = key.into();
        Ok(Some(
            Self::query_first(
                connection,
                Self::query_select()
                    .where_eq("snapshot_id", snapshot)
                    .and()
                    .where_eq("key", key)
                    .build()?,
            )
            .await?,
        ))
    }

    /// Find Metadata by SHA for a Snapshot
    pub async fn find_by_sha<'a, T>(
        connection: &'a T,
        sha: impl Into<String>,
    ) -> Result<Option<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let sha = sha.into();
        if sha.is_empty() {
            return Ok(None);
        }

        Ok(Some(
            Self::query_first(
                connection,
                Self::query_select()
                    .where_eq("key", "bom.sha")
                    .and()
                    .where_eq("value", sha)
                    .build()?,
            )
            .await?,
        ))
    }

    /// Get the value as String
    pub fn as_string(&self) -> String {
        std::str::from_utf8(&self.value).unwrap().to_string()
    }

    /// Convert the bytes value to i32
    pub fn as_i32(&self) -> i32 {
        self.as_string().parse().unwrap_or_default()
    }

    /// Convert the value into a u32
    pub fn as_u32(&self) -> u32 {
        self.as_string().parse().unwrap_or_default()
    }
}
