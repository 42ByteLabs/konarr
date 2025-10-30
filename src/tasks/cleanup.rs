//! # Cleanup task

use crate::models::settings::SettingType;
use crate::models::{ServerSettings, Setting, Snapshot};

use super::TaskTrait;
use chrono::TimeDelta;
use geekorm::{Connection, ConnectionManager, prelude::*};

/// Catalogue the components task
#[derive(Default)]
pub struct CleanupTask {
    force: bool,
}

#[async_trait::async_trait]
impl TaskTrait for CleanupTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        let cleanup =
            ServerSettings::fetch_by_name(&database.acquire().await, Setting::Cleanup).await?;

        if self.force || cleanup.boolean() {
            log::info!("Starting Cleanup Task");
            let timer = self.fetch_timer(&database.acquire().await).await?;
            log::info!("Cleanup Timer: {} days", timer);

            let mut snapshots = Snapshot::all(&database.acquire().await).await?;
            log::info!("Found `{}` snapshots", snapshots.len());

            self.cleanup_snapshots(&database.acquire().await, &mut snapshots, timer)
                .await?;
        } else {
            log::info!("Cleanup Task is disabled");
        }

        Ok(())
    }
}

impl CleanupTask {
    /// Set the Cleanup Task to force
    pub fn force() -> Self {
        Self { force: true }
    }

    /// Fetch the Cleanup Timer
    pub async fn fetch_timer(
        &self,
        connection: &Connection<'_>,
    ) -> Result<TimeDelta, crate::KonarrError> {
        match ServerSettings::fetch_by_name(connection, Setting::CleanupTimer).await {
            Ok(timer_setting) => Ok(TimeDelta::days(
                timer_setting.integer().unwrap_or(90_i64),
            )),
            Err(_) => {
                // Create a default timer
                let mut setting = ServerSettings::new(
                    Setting::CleanupTimer,
                    SettingType::Integer,
                    "90".to_string(),
                );
                setting.save(connection).await?;

                log::warn!("Cleanup Timer not found, using default: 90 days");
                Ok(TimeDelta::days(90))
            }
        }
    }

    /// Cleanup the snapshots
    pub async fn cleanup_snapshots(
        &self,
        connection: &Connection<'_>,
        snapshots: &mut [Snapshot],
        timer: TimeDelta,
    ) -> Result<(), crate::KonarrError> {
        for snatshot in snapshots.iter_mut() {
            log::debug!("Checking snapshot: {}", snatshot.id);

            // Check the last time the snapshot was updated
            if let Some(updated) = snatshot.updated_at {
                log::debug!("Snapshot updated at: {}", updated);
                if updated < chrono::Utc::now() - timer {
                    log::info!("Deleting snapshot: {}", snatshot.id);
                    snatshot.delete(connection).await?;
                }
            } else if snatshot.created_at < chrono::Utc::now() - timer {
                log::info!("Deleting snapshot: {}", snatshot.id);
                snatshot.delete(connection).await?;
            }
        }

        Ok(())
    }
}
