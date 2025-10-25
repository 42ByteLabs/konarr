//! Projects Task
use async_trait::async_trait;
use geekorm::ConnectionManager;

use crate::KonarrError;
use crate::models::dependencies::snapshots::SnapshotState;
use crate::models::{ProjectSnapshots, Projects, SnapshotMetadataKey};

use super::TaskTrait;

/// Projects Task
#[derive(Default)]
pub struct ProjectsTask;

#[async_trait]
impl TaskTrait for ProjectsTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let connection = database.acquire().await;
        log::info!("Task - Running Projects");

        let mut projects = Projects::fetch_servers(&connection).await?;

        for project in projects.iter_mut() {
            update_grouped_projects(&connection, project).await?;

            log::info!("Setting Project '{}' Snapshots as Stale", project.name);
            ProjectSnapshots::set_stale(&connection, project.id).await?;
        }

        log::debug!(
            "Task - Running Projects - Actions :: {}",
            connection.count()
        );

        Ok(())
    }
}

/// Update Grouped Projects
async fn update_grouped_projects(
    connection: &geekorm::Connection<'_>,
    project: &mut Projects,
) -> Result<(), crate::KonarrError> {
    let mut dependencies = 0;
    let mut security_total = 0;

    for child in project.children.iter_mut() {
        if let Some(meta) = child.get_metadata(SnapshotMetadataKey::DependenciesTotal) {
            dependencies += meta.as_i32();
            log::debug!(
                "Project '{}' adding '{}' dependencies",
                child.name,
                meta.as_i32()
            );
        } else if let Some(meta) = child.get_metadata(SnapshotMetadataKey::SecurityAlertTotal) {
            security_total += meta.as_i32();
            log::debug!(
                "Project '{}' adding '{}' security issues",
                child.name,
                meta.as_i32()
            );
        }
    }

    let name = project.name.clone();
    log::debug!("Project '{}' has '{}' dependencies", name, dependencies);

    let Some(mut snap) = project.latest_snapshot() else {
        log::error!("No Snapshot Found");
        return Err(KonarrError::UnknownError("No Snapshot Found".to_string()));
    };

    // Summary of the grouped project (without SBOM)
    if !snap.has_sbom() {
        snap.set_state(connection, SnapshotState::Summary).await?;

        snap.update_metadata(
            connection,
            SnapshotMetadataKey::DependenciesTotal,
            dependencies,
        )
        .await?;

        snap.update_metadata(
            connection,
            SnapshotMetadataKey::SecurityAlertTotal,
            security_total,
        )
        .await?;
    }

    Ok(())
}
