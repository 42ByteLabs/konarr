//! This module contains the tasks that are run by the CLI.

use async_trait::async_trait;
use geekorm::GeekConnection;
use log::info;
use std::sync::Arc;
use tokio::spawn;
use tokio_schedule::Job;

pub mod advisories;
pub mod alerts;
pub mod statistics;

pub use advisories::sync_advisories;
pub use alerts::alert_calculator;
pub use statistics::statistics;

use crate::Config;

/// Initialse background tasks
///
/// Setup a timer to run every 1 minute to do the following:
/// - Calculate statistics
pub async fn init(
    _config: &Config,
    database: Arc<libsql::Database>,
) -> Result<(), crate::KonarrError> {
    info!("Initializing Background Tasks...");

    let tasks = tokio_schedule::every(60).seconds().perform(move || {
        let database = Arc::clone(&database);
        let connection = database.connect().unwrap();
        log::info!("Running Background Tasks");

        async move {
            alert_calculator(&connection)
                .await
                .map_err(|e| log::error!("Task Error :: {}", e))
                .unwrap();

            statistics(&connection)
                .await
                .map_err(|e| log::error!("Task Error :: {}", e))
                .unwrap();
        }
    });
    spawn(tasks);

    Ok(())
}

/// Task Trait
#[async_trait]
pub trait TaskTrait<'a, C>
where
    C: GeekConnection<Connection = C> + 'a,
    Self: Sized,
{
    /// Initialize the Task
    #[allow(unused_variables)]
    async fn init(connection: &'a C) -> Result<bool, crate::KonarrError> {
        Ok(true)
    }

    /// Run the task
    #[allow(unused_variables)]
    async fn run(connection: &'a C) -> Result<(), crate::KonarrError>;

    /// Finish / Done / Completed the tasks
    #[allow(unused_variables)]
    async fn done(connection: &'a C) -> Result<(), crate::KonarrError> {
        Ok(())
    }
}
