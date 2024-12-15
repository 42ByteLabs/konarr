//! # Tasks - Statistics
use geekorm::{GeekConnection, GeekConnector, QueryBuilderTrait};

use crate::models::{Component, ComponentType, Projects, ServerSettings, Setting, Users};

/// Calculate Statistics Task
pub async fn statistics<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
{
    log::info!("Task - Calculating Statistics");
    user_statistics(connection).await?;
    project_statistics(connection).await?;
    dependencies_statistics(connection).await?;

    Ok(())
}

/// User Statistics Task
pub async fn user_statistics<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
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
pub async fn project_statistics<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
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
pub async fn dependencies_statistics<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
{
    ServerSettings::update_statistic(
        connection,
        Setting::StatsDependenciesTotal,
        Component::total(connection).await?,
    )
    .await?;

    // Count the number of components that are programming languages
    ServerSettings::update_statistic(
        connection,
        Setting::StatsDependenciesLanguages,
        Component::row_count(
            connection,
            Component::query_count()
                .where_eq("component_type", ComponentType::ProgrammingLanguage)
                .build()?,
        )
        .await?,
    )
    .await?;

    Ok(())
}
