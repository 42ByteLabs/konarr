//! # Project Models

use chrono::{DateTime, Utc};
use geekorm::{Connection, prelude::*};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use super::{Dependencies, Snapshot};

/// Status of the Project
#[derive(Data, Debug, Default, Clone, PartialEq)]
pub enum ProjectStatus {
    /// Active
    #[default]
    Active,
    /// Inactive
    Inactive,
    /// Archived
    Archived,
}

/// Project Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Projects {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Project Name acts as a unique identifier for the project
    #[geekorm(unique)]
    pub name: String,
    /// Project Title is a the human readable name of the project (can be the same as the name)
    pub title: Option<String>,

    /// Project Description
    pub description: Option<String>,
    /// Project Type
    pub project_type: ProjectType,
    /// Status
    #[geekorm(new = "ProjectStatus::Active")]
    pub status: ProjectStatus,

    /// Parent Project
    #[geekorm(new = "0")]
    pub parent: i32,

    /// Children of the Project
    #[geekorm(skip)]
    #[serde(skip)]
    pub children: Vec<Projects>,

    /// Project Snapshots
    #[geekorm(skip)]
    #[serde(skip)]
    pub snapshots: Vec<Snapshot>,

    /// Datetime Created
    #[geekorm(new = "Utc::now()")]
    pub created_at: DateTime<Utc>,
}

impl Projects {
    /// Get all Projects
    pub async fn all_active(
        connection: &Connection<'_>,
        page: &Page,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .order_by("name", QueryOrder::Desc)
                .page(page)
                .build()?,
        )
        .await?;

        for proj in projects.iter_mut() {
            proj.fetch_children(connection).await?;
            proj.fetch_snapshots(connection).await?;
        }

        Ok(projects)
    }

    /// Count the active Projects
    pub async fn count_active(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_ne("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }

    /// Count the Archived Projects
    pub async fn count_archived(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }

    /// Count the Inactive Projects
    pub async fn count_inactive(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Inactive)
                .build()?,
        )
        .await?)
    }

    /// Count the number of Servers
    pub async fn count_servers(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("project_type", ProjectType::Server)
                .and()
                .where_eq("status", ProjectStatus::Active)
                .build()?,
        )
        .await?)
    }

    /// Count of number of Projects
    pub async fn count_containers(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("project_type", ProjectType::Container)
                .and()
                .where_eq("status", ProjectStatus::Active)
                .build()?,
        )
        .await?)
    }

    /// Search for Projects
    pub async fn search_title(
        connection: &Connection<'_>,
        search: impl Into<String>,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        let search = search.into();

        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_like("title", format!("%{}%", search))
                .build()?,
        )
        .await?;
        for proj in projects.iter_mut() {
            proj.fetch_children(connection).await?;
            proj.fetch_snapshots(connection).await?;
        }
        Ok(projects)
    }

    /// Get Top-Level Projects and their children
    pub async fn fetch_top_level(
        connection: &Connection<'_>,
        page: &Page,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        debug!("Fetching top level projects");

        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_eq("parent", 0)
                .order_by("created_at", QueryOrder::Desc)
                .page(page)
                .build()?,
        )
        .await?;

        for proj in projects.iter_mut() {
            proj.fetch_children(connection).await?;
            proj.fetch_latest_snapshot(connection).await?;
        }

        Ok(projects)
    }

    /// Fetch active projects by type
    pub async fn fetch_project_type(
        connection: &Connection<'_>,
        project_type: impl Into<ProjectType>,
        page: &Page,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        let project_type = project_type.into();
        log::debug!("Fetching Projects by Type: {:?}", project_type);

        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_eq("project_type", project_type)
                .order_by("created_at", QueryOrder::Desc)
                .page(page)
                .build()?,
        )
        .await?;
        for proj in projects.iter_mut() {
            proj.fetch_children(connection).await?;
            proj.fetch_latest_snapshot(connection).await?;
        }

        Ok(projects)
    }

    /// Find a list of projects by component in latest snapshot
    pub async fn find_project_by_component(
        connection: &Connection<'_>,
        component_id: i32,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        log::debug!("Finding Projects by Component: {:?}", component_id);

        let mut results = vec![];
        // TODO: This is a terrible way to do this
        let mut projects = Projects::query(connection, Projects::query_all()).await?;

        for proj in projects.iter_mut() {
            proj.fetch_latest_snapshot(connection).await?;

            if let Some(snap) = proj.snapshots.last() {
                let dep = Dependencies::query(
                    connection,
                    Dependencies::query_select()
                        .where_eq("snapshot_id", snap.id)
                        .and()
                        .where_eq("component_id", component_id)
                        .limit(1)
                        .build()?,
                )
                .await?;

                if dep.len() == 1 {
                    results.push(proj.clone());
                }
            }
        }

        Ok(results)
    }

    /// Find all the possible parents
    pub async fn find_parents(
        connection: &Connection<'_>,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        debug!("Finding all parent projects");
        Ok(Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_eq("project_type", ProjectType::Server)
                .order_by("name", QueryOrder::Asc)
                .build()?,
        )
        .await?)
    }

    /// Get the projects children
    pub async fn fetch_children(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<(), crate::KonarrError> {
        debug!("Fetching Children for Project: {:?}", self.id);

        self.children = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_eq("parent", self.id)
                .order_by("created_at", QueryOrder::Desc)
                .build()?,
        )
        .await?;

        for child in self.children.iter_mut() {
            child.fetch_latest_snapshot(connection).await?;
        }

        Ok(())
    }

    /// Fetch latest Snapshot
    pub async fn fetch_latest_snapshot(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<Option<&mut Snapshot>, crate::KonarrError> {
        log::debug!("Fetching Latest Snapshot for Project: {:?}", self.id);
        match ProjectSnapshots::query_first(
            connection,
            ProjectSnapshots::query_select()
                .where_eq("project_id", self.id)
                .order_by("snapshot_id", QueryOrder::Desc)
                .limit(1)
                .build()?,
        )
        .await
        {
            Ok(snap) => {
                log::debug!("Snapshot ID: {} - {:?}", snap.id, snap.snapshot_id);

                match Snapshot::fetch_by_primary_key(connection, snap.snapshot_id).await {
                    Ok(mut snapshot) => {
                        snapshot.fetch_metadata(connection).await?;

                        self.snapshots.push(snapshot);
                        Ok(self.snapshots.last_mut())
                    }
                    Err(geekorm::Error::SerdeError(err)) => {
                        log::error!("Error fetching Snapshot: {:#?}", err);
                        Err(crate::KonarrError::DatabaseError {
                            backend: connection.to_string(),
                            error: err,
                        })
                    }
                    Err(err) => {
                        log::warn!("Error fetching Snapshot: {:?}", err);
                        Ok(None)
                    }
                }
            }
            Err(err) => {
                log::warn!("Error fetching Snapshot: {:?}", err);
                Ok(None)
            }
        }
    }

    /// Add snapshot to project
    pub async fn add_snapshot(
        &mut self,
        connection: &Connection<'_>,
        snapshot: Snapshot,
    ) -> Result<(), crate::KonarrError> {
        debug!("Adding Snapshot to Project: {:?}", self.id);
        let mut snap = ProjectSnapshots {
            project_id: self.id.into(),
            snapshot_id: snapshot.id.into(),
            ..Default::default()
        };

        snap.save(connection).await?;

        self.snapshots.push(snapshot);

        Ok(())
    }
    /// Fetch Snapshots for the Project
    pub async fn fetch_snapshots(
        &mut self,
        connection: &Connection<'_>,
    ) -> Result<(), crate::KonarrError> {
        log::debug!("Fetching Snapshots for Project: {:?}", self.id);
        let snaps = ProjectSnapshots::fetch_by_project_id(connection, self.id).await?;

        for snap in snaps {
            let mut snaps = Snapshot::fetch_by_primary_key(connection, snap.snapshot_id).await?;
            snaps.fetch_metadata(connection).await?;

            self.snapshots.push(snaps);
        }
        Ok(())
    }

    /// Calculate Alerts for all projects with snapshots
    pub async fn calculate_alerts(
        connection: &Connection<'_>,
        projects: &mut Vec<Self>,
    ) -> Result<(), crate::KonarrError> {
        for project in projects.iter_mut() {
            project.fetch_latest_snapshot(connection).await?;

            if let Some(snapshot) = project.snapshots.last() {
                let mut snapshot = snapshot.clone();

                match project.project_type {
                    ProjectType::Container => {
                        snapshot.calculate_alerts_summary(connection).await?;
                    }
                    _ => {
                        warn!(
                            "Project Type not supported for alerts: {:?}",
                            project.project_type
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Archive the Project
    pub async fn archive(&mut self, connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        self.status = ProjectStatus::Archived;
        self.update(connection).await.map_err(|e| e.into())
    }
}

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

/// Project Type
#[derive(Data, Debug, Default, Clone, PartialEq)]
pub enum ProjectType {
    /// Group of Projects
    #[geekorm(aliases = "group,groups")]
    Group,
    /// Single Application
    #[default]
    #[geekorm(aliases = "app,application,applications")]
    Application,
    /// Server
    #[geekorm(aliases = "server,servers")]
    Server,
    /// Cluster (Kubernetes, Docker Swarm, etc.)
    #[geekorm(aliases = "cluster")]
    Cluster,
    /// Container
    #[geekorm(aliases = "container,containers,docker")]
    Container,
}
