//! This module contains the tasks that are run by the CLI.

use async_trait::async_trait;
use geekorm::GeekConnection;

pub mod advisories;
pub mod alerts;

/// Setup background tasks
// pub async fn setup<T>(config: &Config, connection: Arc<Mutex<T>>) -> Result<(), crate::KonarrError>
// where
//     T: GeekConnection<Connection = T> + 'static + Send + Sync,
// {
//     let alerts_task = every(30).seconds().perform(|| async move {
//         let conn = connection.lock().await;
//         let c = *conn;
//
//         match alerts::alert_calculator(&c).await {
//             Ok(r) => {
//                 debug!("Task complete: {:?}", r);
//             }
//             Err(e) => {
//                 warn!("Task - Error: {:?}", e);
//             }
//         };
//     });
//     spawn(alerts_task);
//
//     Ok(())
// }

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
