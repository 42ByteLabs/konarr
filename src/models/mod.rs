//! # Konarr Models

use geekorm::ConnectionManager;

pub mod auth;
pub mod cache;
pub mod components;
pub mod dependencies;
pub mod projects;
pub mod security;
pub mod settings;

pub use auth::sessions::{SessionState, SessionType, Sessions};
pub use auth::users::{UserRole, Users};
pub use cache::DbCache;
pub use components::{Component, ComponentManager, ComponentType, ComponentVersion};
pub use dependencies::Dependencies;
pub use dependencies::snapshots::{Snapshot, SnapshotMetadata, SnapshotMetadataKey};
pub use projects::{ProjectSnapshots, ProjectStatus, ProjectType, Projects};
pub use security::advisories::AdvisoriesMetadata;
pub use security::{Advisories, Alerts};
pub use settings::{ServerSettings, Setting};

use crate::tasks::TaskTrait;
use crate::tasks::alerts::AlertCalculatorTask;
use crate::tasks::projects::ProjectsTask;
use crate::tasks::statistics::StatisticsTask;
use crate::{Config, KonarrError, db};

/// Initialize the database with the necessary tables.
///
/// - Create / Migrate Database
/// - Initiale data
/// - Update Statistics
/// - Update Security Data
pub async fn database_initialise(config: &mut Config) -> Result<ConnectionManager, KonarrError> {
    log::info!("Initialising Database");
    let database = config.database().await?;

    db::init(&database.acquire().await).await?;

    {
        log::debug!("Initialising Models...");
        let connection = database.acquire().await;
        // Initialise the models
        ServerSettings::init(&connection).await?;
        Component::init(&connection).await?;
        SnapshotMetadata::init(&connection).await?;

        // Store the server setting into the config file
        if let Ok(token) = ServerSettings::fetch_by_name(&connection, Setting::AgentKey).await {
            if config.agent.token != Some(token.value.clone()) {
                log::info!("Updating Agent Token");
                config.agent.token = Some(token.value);
                config.autosave()?;
            }
        }
        log::debug!("Initialised Models :: {}", connection.count());
    }

    // Update Stats
    StatisticsTask::spawn(&database).await?;
    ProjectsTask::spawn(&database).await?;
    AlertCalculatorTask::spawn(&database).await?;

    log::info!("Database Initialised!");
    Ok(database)
}
