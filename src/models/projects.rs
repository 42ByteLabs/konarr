//! # Project Models

use geekorm::prelude::*;

use chrono::{DateTime, Utc};
use log::debug;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use super::Snapshot;

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

    /// Project Name
    #[geekorm(unique)]
    pub name: String,
    /// Project Title
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
    /// Initialize the Projects Table
    pub async fn init<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        debug!("Creating Projects Table and Default Project");
        Projects::create_table(connection).await?;

        // Create a Default Project
        let mut main_server = Projects::new("Main Server", ProjectType::Server);
        main_server.fetch_or_create(connection).await?;

        debug!("Server Project Created: {:?}", main_server);

        match Projects::fetch_by_name(connection, "Main Container").await {
            Ok(_) => return Ok(()),
            Err(_) => {
                let mut container_project = Projects::new("Main Container", ProjectType::Container);
                container_project.parent = main_server.id.into();

                container_project.save(connection).await?;
                debug!("Container Project Created: {:?}", container_project);
            }
        };

        Ok(())
    }

    /// Get all Projects
    pub async fn all<'a, T>(
        connection: &'a T,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .order_by("created_at", QueryOrder::Desc)
                .limit(limit)
                .offset(offset)
                .build()?,
        )
        .await?)
    }

    /// Count the active Projects
    pub async fn count_active<'a, T>(connection: &'a T) -> Result<i64, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_ne("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }

    /// Count the Archived Projects
    pub async fn count_archived<'a, T>(connection: &'a T) -> Result<i64, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }

    /// Count the Inactive Projects
    pub async fn count_inactive<'a, T>(connection: &'a T) -> Result<i64, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Inactive)
                .build()?,
        )
        .await?)
    }

    /// Count the number of Servers
    pub async fn count_servers<'a, T>(connection: &'a T) -> Result<i64, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("project_type", ProjectType::Server)
                .build()?,
        )
        .await?)
    }

    /// Count of number of Projects
    pub async fn count_containers<'a, T>(connection: &'a T) -> Result<i64, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("project_type", ProjectType::Container)
                .build()?,
        )
        .await?)
    }

    /// Search for Projects
    pub async fn search<'a, T>(
        connection: &'a T,
        search: impl Into<String>,
    ) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let search = search.into();

        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_like("name", format!("%{}%", search))
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
    pub async fn fetch_top_level<'a, T>(
        connection: &'a T,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        debug!("Fetching top level projects");

        let mut projects = Projects::query(
            connection,
            Projects::query_select()
                .where_eq("status", ProjectStatus::Active)
                .and()
                .where_eq("parent", 0)
                .order_by("created_at", QueryOrder::Desc)
                .limit(limit)
                .offset(offset)
                .build()?,
        )
        .await?;

        for proj in projects.iter_mut() {
            proj.fetch_children(connection).await?;
            proj.fetch_snapshots(connection).await?;
        }

        Ok(projects)
    }

    /// Find all the possible parents
    pub async fn find_parents<'a, T>(connection: &'a T) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
    pub async fn fetch_children<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
            child.fetch_snapshots(connection).await?;
        }

        Ok(())
    }

    /// Fetch latest Snapshot
    pub async fn fetch_latest_snapshot<'a, T>(
        &self,
        connection: &'a T,
    ) -> Result<Option<Snapshot>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        match ProjectSnapshots::query_first(
            connection,
            ProjectSnapshots::query_select()
                .order_by("created_at", QueryOrder::Desc)
                .limit(1)
                .build()?,
        )
        .await
        {
            Ok(snap) => Ok(Some(
                Snapshot::fetch_by_primary_key(connection, snap.snapshot_id).await?,
            )),
            Err(_) => Ok(None),
        }
    }

    /// Add snapshot to project
    pub async fn add_snapshot<'a, T>(
        &mut self,
        connection: &'a T,
        snapshot: Snapshot,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
    pub async fn fetch_snapshots<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snaps = ProjectSnapshots::fetch_by_project_id(connection, self.id).await?;

        for snap in snaps {
            let mut snaps = Snapshot::fetch_by_primary_key(connection, snap.snapshot_id).await?;
            snaps.fetch_metadata(connection).await?;

            self.snapshots.push(snaps);
        }
        Ok(())
    }

    /// Archive the Project
    pub async fn archive<'a, T>(&mut self, connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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
#[derive(Data, Debug, Default, Clone)]
pub enum ProjectType {
    /// Group of Projects
    Group,
    /// Single Application
    #[default]
    Application,
    /// Server
    Server,
    /// Cluster (Kubernetes, Docker Swarm, etc.)
    Cluster,
    /// Container
    Container,
}

impl Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Group => write!(f, "Group"),
            ProjectType::Application => write!(f, "Application"),
            ProjectType::Server => write!(f, "Server"),
            ProjectType::Cluster => write!(f, "Cluster"),
            ProjectType::Container => write!(f, "Container"),
        }
    }
}

impl From<String> for ProjectType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "group" => ProjectType::Group,
            "application" => ProjectType::Application,
            "server" => ProjectType::Server,
            "cluster" => ProjectType::Cluster,
            "container" => ProjectType::Container,
            _ => ProjectType::Application,
        }
    }
}

impl From<&String> for ProjectType {
    fn from(s: &String) -> Self {
        ProjectType::from(s.clone())
    }
}
