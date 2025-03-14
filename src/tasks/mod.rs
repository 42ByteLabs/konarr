//! This module contains the tasks that are run by the CLI.

use std::sync::Arc;
use tokio::spawn;
use tokio_schedule::Job;

use geekorm::{Connection, ConnectionManager};

pub mod advisories;
pub mod alerts;
pub mod catalogue;
pub mod statistics;

pub use advisories::{AdvisoriesTask, sync_advisories};
pub use alerts::AlertCalculatorTask;
pub use catalogue::catalogue;
pub use statistics::StatisticsTask;

use crate::Config;
use crate::models::{ServerSettings, Setting};

/// Initialse background tasks
///
/// Setup a timer to run every hour to do the following:
/// - Sync advisories
/// - Calculate statistics
/// - Calculate alerts
pub async fn init<'a>(
    config: Arc<Config>,
    database: &'a ConnectionManager,
) -> Result<(), crate::KonarrError> {
    log::info!("Initializing Background Tasks...");

    let database = database.clone();

    let tasks = tokio_schedule::every(60).minutes().perform(move || {
        let database = database.clone();
        let config = Arc::clone(&config);

        log::info!("Running Background Tasks");

        async move {
            let connection = database.acquire().await;

            match sync_advisories(&config, &connection).await {
                Ok(_) => log::debug!("Advisories Synced"),
                Err(e) => log::error!("Advisories Sync Error: {}", e),
            }

            let rescan = ServerSettings::fetch_by_name(&connection, Setting::SecurityRescan)
                .await
                .map_err(|e| {
                    log::error!("Task Error :: {}", e);
                });

            if let Ok(mut rescan) = rescan {
                if rescan.boolean() {
                    log::info!("Rescanning Projects");
                    // Reset the flag to disabled before we perform the scan
                    if let Err(e) = rescan.set_update(&connection, "disabled").await {
                        log::error!("Error resetting rescan flag: {}", e);
                    }

                    if let Err(e) = advisories::scan(&config, &connection).await {
                        log::error!("Error rescanning projects: {}", e);
                    }
                }
            }

            AlertCalculatorTask::task(&connection)
                .await
                .map_err(|e| log::error!("Task Error :: {}", e))
                .unwrap();

            StatisticsTask::task(&connection)
                .await
                .map_err(|e| log::error!("Task Error :: {}", e))
                .unwrap();
        }
    });
    spawn(tasks);

    Ok(())
}

/// Task Trait
#[async_trait::async_trait]
pub trait TaskTrait
where
    Self: Sized + Default + Send + Sync,
{
    /// Initialize the Task
    #[allow(unused_variables)]
    async fn init(connection: &Connection<'_>) -> Result<Self, crate::KonarrError> {
        Ok(Self::default())
    }

    /// Run the task
    #[allow(unused_variables)]
    async fn run(&self, connection: &Connection<'_>) -> Result<(), crate::KonarrError>;

    /// Finish / Done / Completed the tasks
    #[allow(unused_variables)]
    async fn done(&self, connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        Ok(())
    }

    /// Run the task with a connection
    async fn task(connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        let task = Self::init(connection).await?;
        task.run(connection).await?;
        task.done(connection).await?;
        Ok(())
    }

    /// Spawn and run the task as a background task
    async fn spawn(database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let database = database.clone();
        tokio::spawn(async move {
            let name = std::any::type_name::<Self>();
            let connection = database.acquire().await;
            log::info!("Spawed Task :: {}", name);

            Self::task(&connection)
                .await
                .map_err(|e| {
                    log::error!("Failed to run alert calculator: {:?}", e);
                })
                .ok();
            log::debug!("Task - {} - {} transactions", name, connection.count());
            log::info!("Spawed Task Completed :: {}", name);
        });
        Ok(())
    }
}
