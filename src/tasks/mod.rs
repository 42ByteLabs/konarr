//! This module contains the tasks that are run by the CLI.

use geekorm::{Connection, ConnectionManager};
use std::sync::Arc;
use tokio::spawn;
use tokio_schedule::Job;

pub mod advisories;
#[cfg(feature = "tools-grypedb")]
pub mod advisories_sync;
pub mod alerts;
pub mod catalogue;
pub mod cleanup;
pub mod projects;
pub mod sbom;
pub mod server;
pub mod statistics;

pub use advisories::AdvisoriesTask;
#[cfg(feature = "tools-grypedb")]
pub use advisories_sync::AdvisoriesSyncTask;
pub use alerts::AlertCalculatorTask;
pub use catalogue::CatalogueTask;
pub use projects::ProjectsTask;
pub use statistics::StatisticsTask;

use crate::{Config, tasks::server::ServerTask};
use cleanup::CleanupTask;

/// Initialse background tasks
///
/// Setup a timer to run every hour to do the following:
/// - Sync advisories
/// - Calculate statistics
/// - Calculate alerts
pub async fn init(
    config: Arc<Config>,
    database: &ConnectionManager,
) -> Result<(), crate::KonarrError> {
    log::info!("Initializing Background Tasks...");

    let hourlydb = database.clone();
    let hourly_config = Arc::clone(&config);

    let hourly = tokio_schedule::every(60).minutes().perform(move || {
        let database = hourlydb.clone();
        let config = Arc::clone(&hourly_config);
        log::info!("Running Hourly Background Tasks");

        async move {
            if let Ok(task) = AdvisoriesTask::new(&config) {
                if let Err(e) = task.run(&database).await {
                    log::error!("Task Error :: {}", e);
                }
            }

            if let Err(e) = ServerTask::task(&database).await {
                log::error!("Task Error :: {}", e);
            }

            if let Err(e) = AdvisoriesSyncTask::spawn(&database).await {
                log::error!("Task Error :: {}", e);
            }

            if let Err(e) = AlertCalculatorTask::task(&database).await {
                log::error!("Task Error :: {}", e);
            }

            if let Err(e) = ProjectsTask::task(&database).await {
                log::error!("Task Error :: {}", e);
            }

            if let Err(e) = StatisticsTask::task(&database).await {
                log::error!("Task Error :: {}", e);
            }

            if let Err(e) = CleanupTask::task(&database).await {
                log::error!("Task Error :: {}", e);
            }
        }
    });
    spawn(hourly);

    Ok(())
}

/// Task Trait
#[async_trait::async_trait]
pub trait TaskTrait
where
    Self: Sized + Default + Send + Sync + 'static,
{
    /// Initialize the Task
    #[allow(unused_variables)]
    async fn init(connection: &Connection<'_>) -> Result<Self, crate::KonarrError> {
        Ok(Self::default())
    }

    /// Run the task
    #[allow(unused_variables)]
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError>;

    /// Finish / Done / Completed the tasks
    #[allow(unused_variables)]
    async fn done(&self, connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        Ok(())
    }

    /// Run the task with a connection
    async fn task(database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let task = Self::init(&database.acquire().await).await?;
        task.run(database).await?;
        task.done(&database.acquire().await).await?;
        Ok(())
    }

    /// Spawn and run the task as a background task
    async fn spawn(database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let database = database.clone();
        tokio::spawn(async move {
            let name = std::any::type_name::<Self>();
            log::info!("Spawned Task :: {}", name);

            Self::task(&database)
                .await
                .map_err(|e| {
                    log::error!("Failed to run alert calculator: {:?}", e);
                })
                .ok();
            log::info!("Spawned Task Completed :: {}", name);
        });
        Ok(())
    }

    /// Spawn and run the task as a background task
    async fn spawn_task(self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let database = database.clone();
        tokio::spawn(async move {
            let name = std::any::type_name::<Self>();
            log::info!("Spawned Task :: {}", name);

            self.run(&database)
                .await
                .map_err(|e| {
                    log::error!("Failed to run alert calculator: {:?}", e);
                })
                .ok();
            log::info!("Spawned Task Completed :: {}", name);
        });
        Ok(())
    }
}
