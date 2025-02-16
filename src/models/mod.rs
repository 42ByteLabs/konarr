//! # Konarr Models

use geekorm::prelude::*;
use log::{debug, info};

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
pub use dependencies::snapshots::{Snapshot, SnapshotMetadata, SnapshotMetadataKey};
pub use dependencies::Dependencies;
pub use projects::{ProjectSnapshots, ProjectStatus, ProjectType, Projects};
pub use security::advisories::AdvisoriesMetadata;
pub use security::{Advisories, Alerts};
pub use settings::{ServerSettings, Setting};

use crate::{db, Config, KonarrError};

/// Initialize the database with the necessary tables.
///
/// - Create / Migrate Database
/// - Initiale data
/// - Update Statistics
/// - Update Security Data
pub async fn database_initialise<T>(config: &mut Config, connection: &T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + Sync + Send,
{
    db::init(connection).await?;

    // Initialise the models
    ServerSettings::init(connection).await?;
    Component::init(connection).await?;
    SnapshotMetadata::init(connection).await?;

    // Store the server setting into the config file
    if let Ok(token) = ServerSettings::fetch_by_name(connection, Setting::AgentKey).await {
        if config.agent.token != Some(token.value.clone()) {
            log::info!("Updating Agent Token");
            config.agent.token = Some(token.value);
            config.autosave()?;
        }
    }

    // Update Stats
    crate::tasks::statistics(connection).await?;
    // Calculate Alerts
    crate::tasks::alerts::alert_calculator(connection).await?;

    Ok(())
}
