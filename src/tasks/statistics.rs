//! # Tasks - Statistics
use async_trait::async_trait;
use geekorm::{ConnectionManager, GeekConnector, QueryBuilderTrait};

use crate::models::{Component, ComponentType, ProjectStatus, ProjectType, ServerSettings, Setting, Users};

use super::TaskTrait;

/// Statistics Task
#[derive(Default)]
pub struct StatisticsTask;

#[async_trait]
impl TaskTrait for StatisticsTask {
    /// Calculate Statistics Task
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let connection = database.acquire().await;

        log::info!("Task - Calculating Statistics");
        user_statistics(&connection).await?;
        project_statistics(&connection).await?;
        dependencies_statistics(&connection).await?;

        log::debug!(
            "Task - Calculating Statistics - Actions :: {}",
            connection.count()
        );
        log::debug!("Task - Calculating Statistics - Complete");
        Ok(())
    }
}

/// User Statistics Task
pub async fn user_statistics(
    connection: &geekorm::Connection<'_>,
) -> Result<(), crate::KonarrError> {
    ServerSettings::update_statistic(
        connection,
        Setting::StatsUsersTotal,
        Users::total(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsUsersActive,
        Users::count_active(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsUsersInactive,
        Users::count_inactive(connection).await?,
    )
    .await?;

    Ok(())
}

/// Project Statistics Task
pub async fn project_statistics(
    connection: &geekorm::Connection<'_>,
) -> Result<(), crate::KonarrError> {
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsTotal,
        ProjectStatus::count_active(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsInactive,
        ProjectStatus::count_inactive(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsArchived,
        ProjectStatus::count_archived(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsServers,
        ProjectType::count_servers(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsContainers,
        ProjectType::count_containers(connection).await?,
    )
    .await?;

    Ok(())
}

/// Dependency Statistics Task
pub async fn dependencies_statistics(
    connection: &geekorm::Connection<'_>,
) -> Result<(), crate::KonarrError> {
    ServerSettings::update_statistic(
        connection,
        Setting::StatsDependenciesTotal,
        Component::total(connection).await?,
    )
    .await?;

    let stats = vec![
        (ComponentType::Library, Setting::StatsLibraries),
        (ComponentType::Application, Setting::StatsApplications),
        (ComponentType::Framework, Setting::StatsFrameworks),
        (ComponentType::ProgrammingLanguage, Setting::StatsLanguages),
        (
            ComponentType::OperatingSystem,
            Setting::StatsOperatingSystems,
        ),
        (
            ComponentType::CompressionLibrary,
            Setting::StatsCompressionLibraries,
        ),
        (ComponentType::Database, Setting::StatsDatabases),
        (
            ComponentType::CryptographyLibrary,
            Setting::StatsCryptographicLibraries,
        ),
        (ComponentType::PackageManager, Setting::StatsPackageManagers),
        (
            ComponentType::OperatingEnvironment,
            Setting::StatsOperatingEnvironments,
        ),
        (ComponentType::Middleware, Setting::StatsMiddleware),
    ];

    for (component_type, setting) in stats {
        log::debug!("Calculating Statistics for: {:?}", component_type);
        let count = Component::row_count(
            connection,
            Component::query_count()
                .where_eq("component_type", component_type)
                .build()?,
        )
        .await?;

        ServerSettings::update_statistic(connection, setting, count).await?;
    }
    Ok(())
}
