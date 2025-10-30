//! # Project Snapshots
use chrono::{DateTime, Utc};
use geekorm::{Connection, prelude::*};
use serde::{Deserialize, Serialize};

use crate::models::{Snapshot, SnapshotState};

use super::Projects;

/// Project Snapshots
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshots {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,
    /// Project ID
    #[geekorm(foreign_key = "Projects.id")]
    pub project_id: ForeignKey<i32, Projects>,
    /// Snapshot ID
    #[geekorm(foreign_key = "Snapshot.id")]
    pub snapshot_id: ForeignKey<i32, Snapshot>,

    /// Datetime Created
    #[geekorm(new = "Utc::now()")]
    pub created_at: DateTime<Utc>,
}

impl ProjectSnapshots {
    /// Count the number of snapshots for a given project
    pub async fn count_by_project_id(
        connection: &Connection<'_>,
        project_id: PrimaryKey<i32>,
    ) -> Result<i64, geekorm::Error> {
        ProjectSnapshots::row_count(
            connection,
            ProjectSnapshots::query_count()
                .where_eq("project_id", project_id)
                .build()?,
        )
        .await
    }

    /// Fetch the latest snapshot
    pub async fn fetch_latest(
        connection: &Connection<'_>,
        project_id: PrimaryKey<i32>,
    ) -> Result<Self, geekorm::Error> {
        ProjectSnapshots::query_first(
            connection,
            ProjectSnapshots::query_select()
                .where_eq("project_id", project_id)
                .order_by("snapshot_id", QueryOrder::Desc)
                .limit(1)
                .build()?,
        )
        .await
    }

    /// Fetch all of the snapshots for a given project id that is not the current/latest
    /// snapshot.
    pub async fn fetch_previous_by_project_id(
        connection: &Connection<'_>,
        project_id: PrimaryKey<i32>,
        latest_snapshot_id: PrimaryKey<i32>,
    ) -> Result<Vec<Self>, geekorm::Error> {
        ProjectSnapshots::query(
            connection,
            ProjectSnapshots::query_select()
                .where_eq("project_id", project_id)
                .where_ne("snapshot_id", latest_snapshot_id)
                .order_by("snapshot_id", QueryOrder::Desc)
                .build()?,
        )
        .await
    }

    /// Fetch all snapshots for a given project and mark them as stale
    /// if they are not the latest snapshot.
    pub async fn set_stale(
        connection: &Connection<'_>,
        project_id: impl Into<PrimaryKey<i32>>,
    ) -> Result<(), crate::KonarrError> {
        let project_id = project_id.into();
        let latest = Self::fetch_latest(connection, project_id).await?;
        let snapshots =
            Self::fetch_previous_by_project_id(connection, project_id, latest.id).await?;

        for snapshot in snapshots {
            let mut snap = Snapshot::fetch_by_primary_key(connection, snapshot.snapshot_id).await?;
            if snap.state == SnapshotState::Completed {
                log::debug!(
                    "Marking snapshot {} as stale for project {}",
                    snap.id,
                    project_id
                );
                snap.set_state(connection, SnapshotState::Stale).await?;
            }
        }
        Ok(())
    }
}
