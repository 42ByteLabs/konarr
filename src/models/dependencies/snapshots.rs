//! # Snapshot Model

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use geekorm::prelude::*;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{bom::BillOfMaterials, models::Dependencies};

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
            SnapshotMetadata::update_or_create(
                connection,
                self.id,
                "container.image",
                image.clone(),
            )
            .await?;
            // TODO: Parse the image to get the registry, repository, tag
        }
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
        let key = key.into();
        let value = value.into();
        debug!("Updating Metadata for Snapshot({:?}) :: {} ", snapshot, key);
        Ok(
            match Self::find_by_key(connection, snapshot, key.clone()).await {
                Ok(Some(mut meta)) => {
                    meta.value = value;
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
}
