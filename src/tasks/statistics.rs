//! # Tasks - Statistics
use geekorm::{GeekConnection, GeekConnector, QueryBuilderTrait};

use crate::models::{Component, ComponentType, Projects, ServerSettings, Setting, Users};

/// Calculate Statistics Task
pub async fn statistics<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    log::info!("Task - Calculating Statistics");
    user_statistics(connection).await?;
    project_statistics(connection).await?;
    dependencies_statistics(connection).await?;

    Ok(())
}

/// User Statistics Task
pub async fn user_statistics<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
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
pub async fn project_statistics<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsTotal,
        Projects::count_active(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsInactive,
        Projects::count_inactive(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsArchived,
        Projects::count_archived(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsServers,
        Projects::count_servers(connection).await?,
    )
    .await?;
    ServerSettings::update_statistic(
        connection,
        Setting::StatsProjectsContainers,
        Projects::count_containers(connection).await?,
    )
    .await?;

    Ok(())
}

/// Dependency Statistics Task
pub async fn dependencies_statistics<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
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
