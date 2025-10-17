//! # Project Type
use geekorm::{Connection, prelude::*};

use super::Projects;

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

impl ProjectStatus {
    /// Count Active
    pub async fn count_active(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_ne("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }

    /// Count Inactive
    pub async fn count_inactive(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Inactive)
                .build()?,
        )
        .await?)
    }

    /// Count Archived
    pub async fn count_archived(connection: &Connection<'_>) -> Result<i64, crate::KonarrError> {
        Ok(Projects::row_count(
            connection,
            Projects::query_count()
                .where_eq("status", ProjectStatus::Archived)
                .build()?,
        )
        .await?)
    }
}

impl ProjectType {
    /// Count Servers
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

    /// Count Containers
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
}
