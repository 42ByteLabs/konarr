//! # Project Snapshots
use chrono::{DateTime, Utc};
use geekorm::{Connection, prelude::*};
use serde::{Deserialize, Serialize};

use crate::models::Snapshot;

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
    pub async fn fetch_latest(connection: &Connection<'_>, project_id: PrimaryKey<i32>) -> Result<Self, geekorm::Error> {
        ProjectSnapshots::query_first(
            connection,
            ProjectSnapshots::query_select()
                .where_eq("project_id", project_id)
                .order_by("snapshot_id", QueryOrder::Desc)
                .limit(1)
                .build()?
        ).await
    }
}
