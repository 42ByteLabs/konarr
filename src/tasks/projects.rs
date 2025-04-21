//! Projects Task
use async_trait::async_trait;
use geekorm::{ConnectionManager, GeekConnector};

use crate::KonarrError;
use crate::models::{Projects, Snapshot, SnapshotMetadataKey};

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

    for child in &project.children {
        if let Some(snap) = child.snapshots.first() {
            if let Some(meta) = snap.metadata.get(&SnapshotMetadataKey::DependenciesTotal) {
                dependencies += meta.as_i32();
            }
        }
    }

    if project.snapshots.is_empty() {
        log::info!("Creating Snapshot for Project '{}'", project.name);
        let mut snapshot = Snapshot::new();
        snapshot.save(connection).await?;

        project.add_snapshot(connection, snapshot).await?;
    }

    let mut snap: Snapshot = if let Some(s) = project.snapshots.first() {
        s.clone()
    } else {
        log::error!("No Snapshot Found");
        return Err(KonarrError::UnknownError("No Snapshot Found".to_string()));
    };

    if let Some(meta) = snap.metadata.get(&SnapshotMetadataKey::DependenciesTotal) {
        let meta_deps = meta.as_i32();
        if meta_deps != dependencies {
            log::info!(
                "Updating Project '{}' Dependencies Total: {} -> {}",
                project.name,
                meta_deps,
                dependencies
            );
        } else {
            log::debug!(
                "Project('{}') Dependencies Total: {}",
                project.name,
                dependencies
            );
        }
    } else {
        log::info!(
            "Setting Project '{}' Dependencies Total: {}",
            project.name,
            dependencies
        );
        snap.set_metadata(
            connection,
            SnapshotMetadataKey::DependenciesTotal,
            dependencies.to_string().as_str(),
        )
        .await?;
    }

    Ok(())
}
