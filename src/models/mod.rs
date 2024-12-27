//! # Konarr Models

use geekorm::prelude::*;
use log::debug;

pub mod auth;
pub mod components;
pub mod dependencies;
pub mod projects;
pub mod security;
pub mod settings;

pub use auth::sessions::{SessionState, SessionType, Sessions};
pub use auth::users::{UserRole, Users};
pub use components::{Component, ComponentManager, ComponentType, ComponentVersion};
pub use dependencies::snapshots::{Snapshot, SnapshotMetadata};
pub use dependencies::Dependencies;
pub use projects::{ProjectSnapshots, ProjectStatus, ProjectType, Projects};
pub use security::advisories::AdvisoriesMetadata;
pub use security::{Advisories, Alerts};

pub use settings::{ServerSettings, Setting};

use crate::KonarrError;

/// Initialize the database with the necessary tables.
pub async fn database_create<'a, T>(connection: &'a T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    let connection = connection.into();
    // Session
    debug!("Creating tables");

    debug!("Creating Server Settings table");
    ServerSettings::init(connection).await?;

    debug!("Creating Sessions table");
    Sessions::create_table(connection).await?;
    // Users
    debug!("Creating Users table");
    Users::create_table(connection).await?;

    // Components
    debug!("Creating Components table...");
    ComponentVersion::create_table(connection).await?;
    Component::init(connection).await?;

    debug!("Creating Snapshots table...");
    Snapshot::create_table(connection).await?;
    SnapshotMetadata::init(connection).await?;
    debug!("Creating Dependencies table...");
    Dependencies::create_table(connection).await?;

    debug!("Security tables...");
    Advisories::create_table(connection).await?;
    AdvisoriesMetadata::create_table(connection).await?;
    Alerts::create_table(connection).await?;

    debug!("Creating Projects tables...");
    Projects::init(connection).await?;
    ProjectSnapshots::create_table(connection).await?;

    Ok(())
}
