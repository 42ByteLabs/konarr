//! # Task to Sync Security Advisories
use async_trait::async_trait;
use geekorm::{Connection, ConnectionManager};
use log::{debug, error, info, warn};
use std::path::PathBuf;

use crate::KonarrError;
use crate::models::{ServerSettings, Setting};
use crate::utils::grypedb::GrypeDatabase;

use super::TaskTrait;

/// Advisories Sync Task to sync security advisories
///
/// This currently is done by syncing the Grype Database
#[derive(Default)]
pub struct AdvisoriesSyncTask {
    grype_path: PathBuf,
}

#[async_trait]
impl TaskTrait for AdvisoriesSyncTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        match self.sync_advisories(database).await {
            Ok(_) => debug!("Advisories Synced"),
            Err(e) => error!("Advisories Sync Error: {}", e),
        }
        Ok(())
    }
}

impl AdvisoriesSyncTask {
    /// Poll for Advisories and update the database
    pub async fn sync_advisories(
        &self,
        database: &geekorm::ConnectionManager,
    ) -> Result<(), KonarrError> {
        if !ServerSettings::get_bool(&database.acquire().await, Setting::SecurityAdvisories).await?
        {
            info!("Advisories Disabled");
            return Ok(());
        }

        debug!("Grype Path: {:?}", self.grype_path);

        if ServerSettings::get_bool(
            &database.acquire().await,
            Setting::SecurityAdvisoriesPolling,
        )
        .await?
        {
            let mut updated_last = ServerSettings::fetch_by_name(
                &database.acquire().await,
                Setting::SecurityAdvisoriesUpdated,
            )
            .await?;

            // The last updated time could be blank and unparsable
            if let Ok(last_updated_time) =
                chrono::DateTime::parse_from_rfc3339(updated_last.value.as_str())
            {
                debug!("Last Updated: {}", last_updated_time);
                let now = chrono::Utc::now();
                let delta = last_updated_time
                    .checked_add_signed(chrono::Duration::hours(1))
                    .ok_or(KonarrError::UnknownError("Invalid Date".to_string()))?;
                debug!("Next Update: {} < {}", now, delta);

                // Check if its been 1hr since the last update
                if now < delta {
                    debug!("Advisory DB Updated within the last hour");
                    return Ok(());
                }
            } else {
                debug!("Invalid Advisory Last Updated Time");
            }

            info!("Starting Advisory Database Sync");
            match GrypeDatabase::sync(&self.grype_path).await {
                Ok(new) => {
                    if new {
                        info!("New Advisory Database Synced");

                        let mut grypedb_connection =
                            GrypeDatabase::connect(&self.grype_path).await?;
                        grypedb_connection.fetch_vulnerabilities().await?;

                        info!("Scanning projects for security alerts");
                        info!("Project scanning complete");
                    } else {
                        info!(
                            "Advisory Database Synced but no new advisories, skipping project scanning for security alerts"
                        );
                    }
                }
                Err(e) => {
                    warn!("Advisory Sync Error: {}", e);
                    self.reset_polling(&database.acquire().await).await?;
                }
            };

            updated_last
                .set_update(&database.acquire().await, chrono::Utc::now().to_rfc3339())
                .await?;
        } else {
            debug!("Advisory Polling Disabled");
        }

        let grype = match GrypeDatabase::connect(&self.grype_path).await {
            Ok(db) => db,
            Err(_) => {
                warn!("Errors loading Grype DB");
                return Ok(());
            }
        };

        let connection = database.acquire().await;

        // Set Version
        let grype_id = match grype.fetch_grype().await {
            Ok(grype) => grype,
            Err(_) => {
                warn!("Errors loading Grype DB");
                self.reset_polling(&connection).await?;

                return Ok(());
            }
        };
        ServerSettings::fetch_by_name(&connection, Setting::SecurityAdvisoriesVersion)
            .await?
            .set_update(&connection, grype_id.build_timestamp.to_string().as_str())
            .await?;

        Ok(())
    }

    /// Reset the polling flag if there was an error
    async fn reset_polling(&self, connection: &Connection<'_>) -> Result<(), KonarrError> {
        ServerSettings::fetch_by_name(connection, Setting::SecurityAdvisoriesPolling)
            .await?
            .set_update(connection, "disabled")
            .await?;

        Ok(())
    }
}
