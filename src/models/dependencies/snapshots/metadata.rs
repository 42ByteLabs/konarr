//! # Model - Snapshot Metadata

use chrono::{DateTime, Utc};
use geekorm::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};

use super::Snapshot;

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
    pub key: SnapshotMetadataKey,

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

        let all = match Self::all(connection).await {
            Ok(all) => all,
            Err(e) => {
                log::error!("Failed to get all metadata: {:?}", e);
                log::error!("Please report this error to the Konarr team");
                return Err(e.into());
            }
        };
        log::debug!("Found {} metadata entries", all.len());

        Ok(())
    }

    /// Fetch all Snapshot Metadata
    pub async fn all<'a, T>(connection: &'a T) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Self::query(connection, Self::query_select().build()?).await?)
    }

    /// Update or Create Metadata
    pub async fn update_or_create<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: &SnapshotMetadataKey,
        value: impl Into<Vec<u8>>,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
        let value = value.into();
        debug!("Updating Metadata for Snapshot({:?}) :: {} ", snapshot, key);

        Ok(match Self::find_by_key(connection, snapshot, &key).await {
            Ok(Some(mut meta)) => {
                meta.value = value;
                meta.updated_at = chrono::Utc::now();

                meta.update(connection).await?;
                meta
            }
            _ => Self::add(connection, snapshot, key, value).await?,
        })
    }
    /// Add new Metadata to the Snapshot
    pub async fn add<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: &SnapshotMetadataKey,
        value: impl Into<Vec<u8>>,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
        debug!("Adding Metadata to Snapshot({:?}) :: {} ", snapshot, key);

        let mut meta = Self::new(snapshot, key.clone(), value.into());
        meta.save(connection).await?;
        Ok(meta)
    }

    /// Find Metadata by Key for a Snapshot
    pub async fn find_by_key<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        key: &SnapshotMetadataKey,
    ) -> Result<Option<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshot = snapshot.into();
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
                    .where_eq("key", SnapshotMetadataKey::BomSha)
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

#[derive(Data, Debug, Default, Clone, Hash, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum SnapshotMetadataKey {
    // Operating System Info
    #[geekorm(key = "os")]
    Os,
    #[geekorm(key = "os.version")]
    OsVersion,
    #[geekorm(key = "os.arch")]
    OsArch,
    #[geekorm(key = "os.kernel")]
    OsKernel,

    // OS Container Engine
    #[geekorm(key = "container.engine")]
    ContainerEngine,
    #[geekorm(key = "container.engine.version")]
    ContainerEngineVersion,

    // Container Info
    #[geekorm(key = "container")]
    Container,
    #[geekorm(key = "container.image")]
    ContainerImage,
    /// SHA256 of the container
    #[geekorm(key = "container.sha")]
    ContainerSha,
    #[geekorm(key = "container.version")]
    ContainerVersion,
    /// Container Description provided by the user
    #[geekorm(key = "container.description")]
    ContainerDescription,
    #[geekorm(key = "container.url")]
    ContainerUrl,
    #[geekorm(key = "container.licenses")]
    ContainerLicenses,
    #[geekorm(key = "container.authors")]
    ContainerAuthor,

    // BOM Data
    #[geekorm(key = "bom.type")]
    BomType,
    #[geekorm(key = "bom.version")]
    BomVersion,
    /// BOM SHA (used in the bill of materials, not container)
    #[geekorm(key = "bom.sha")]
    BomSha,
    #[geekorm(key = "bom.tool")]
    BomTool,
    #[geekorm(key = "bom.tool.name")]
    BomToolName,
    #[geekorm(key = "bom.tool.version")]
    BomToolVersion,
    /// Path to where the SBOM is stored
    #[geekorm(key = "bom.path")]
    BomPath,

    // Dependency Info
    #[geekorm(key = "dependencies.total", aliases = "bom.dependencies.count")]
    DependenciesTotal,

    /// If a tool is providing the alerts
    #[geekorm(key = "security.tools.alerts")]
    SecurityToolsAlerts,

    // Security Alert Info
    #[geekorm(
        key = "security.alerts.total",
        aliases = "security.total.count,security.counts.total"
    )]
    SecurityAlertTotal,
    #[geekorm(
        key = "security.alerts.critical",
        aliases = "security.critical.count,security.counts.critical"
    )]
    SecurityAlertCritical,
    #[geekorm(
        key = "security.alerts.high",
        aliases = "security.high.count,security.counts.high"
    )]
    SecurityAlertHigh,
    #[geekorm(
        key = "security.alerts.medium",
        aliases = "security.medium.count,security.counts.medium"
    )]
    SecurityAlertMedium,
    #[geekorm(
        key = "security.alerts.low",
        aliases = "security.low.count,security.counts.low"
    )]
    SecurityAlertLow,
    #[geekorm(
        key = "security.alerts.informational",
        aliases = "security.informational.count,security.counts.informational"
    )]
    SecurityAlertInformational,
    #[geekorm(
        key = "security.alerts.unmaintained",
        aliases = "security.unmaintained.count"
    )]
    SecurityAlertUnmaintained,
    #[geekorm(
        key = "security.alerts.malware",
        aliases = "security.malware.count,security.counts.malware"
    )]
    SecurityAlertMalware,
    #[geekorm(
        key = "security.alerts.unknown",
        aliases = "security.unknown.count,security.counts.unknown"
    )]
    SecurityAlertUnknown,

    #[geekorm(key = "unknown")]
    #[default]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keys() {
        let key = SnapshotMetadataKey::from("os");
        assert_eq!(key, SnapshotMetadataKey::Os);

        let keys = vec!["security.critical.count", "security.counts.critical"];
        for key in keys {
            assert_eq!(
                SnapshotMetadataKey::from(key),
                SnapshotMetadataKey::SecurityAlertCritical
            );
        }
    }
}
