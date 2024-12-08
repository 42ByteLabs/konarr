//! This module contains the tasks that are run by the CLI.

use async_trait::async_trait;
use geekorm::GeekConnection;
use log::info;

pub mod advisories;
pub mod alerts;
pub mod statistics;

/// Calculate Statistics Task
pub async fn statistics<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
{
    info!("Task - Calculating Statistics");
    statistics::user_statistics(connection).await?;
    statistics::project_statistics(connection).await?;
    statistics::dependencies_statistics(connection).await?;

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
