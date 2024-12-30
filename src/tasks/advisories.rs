//! # Task - Advisories

use crate::{
    models::{security::SecurityState, Alerts, Projects, ServerSettings, Setting},
    utils::grypedb::GrypeDatabase,
    Config, KonarrError,
};
use geekorm::prelude::*;
use log::{debug, info, warn};

/// Poll for Advisories and update the database
pub async fn sync_advisories<'a, T>(
    config: &'a Config,
    connection: &'a T,
) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    if !ServerSettings::get_bool(connection, Setting::SecurityAdvisories).await? {
        info!("Advisories Disabled");
        return Ok(());
    }

    let grype_path = config.grype_path()?;
    debug!("Grype Path: {:?}", grype_path);

    if ServerSettings::get_bool(connection, Setting::SecurityAdvisoriesPolling).await? {
        info!("Starting Advisory DB Polling");
        match GrypeDatabase::sync(&grype_path).await {
            Ok(new) => {
                info!("Advisory Sync Complete");

                if new {
                    info!("New Advisory Data");

                    let mut grypedb_connection = GrypeDatabase::connect(&grype_path).await?;
                    grypedb_connection.fetch_vulnerabilities().await?;

                    scan_projects(connection, &grypedb_connection).await?;
                }
            }
            Err(e) => {
                warn!("Advisory Sync Error: {}", e);
                reset_polling(connection).await?;
            }
        };
        ServerSettings::fetch_by_name(connection, Setting::SecurityAdvisoriesUpdated)
            .await?
            .set_update(connection, chrono::Utc::now().to_rfc3339())
            .await?;
    } else {
        debug!("Advisory Polling Disabled");
    }

    let grype = match GrypeDatabase::connect(&grype_path).await {
        Ok(db) => db,
        Err(_) => {
            warn!("Errors loading Grype DB");
            return Ok(());
        }
    };

    // Set Version
    let grype_id = match grype.fetch_grype().await {
        Ok(grype) => grype,
        Err(_) => {
            warn!("Errors loading Grype DB");
            reset_polling(connection).await?;

            return Ok(());
        }
    };
    ServerSettings::fetch_by_name(connection, Setting::SecurityAdvisoriesVersion)
        .await?
        .set_update(connection, grype_id.build_timestamp.to_string().as_str())
        .await?;

    Ok(())
}

/// Scan every project for security alerts
pub async fn scan_projects<'a, T>(
    connection: &'a T,
    grypedb: &GrypeDatabase,
) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    info!("Scanning projects snapshots for security alerts");

    let mut projects = Projects::fetch_all(connection).await?;
    info!("Projects Count: {}", projects.len());
    info!("Vulnerability DB: {}", grypedb.vulnerabilities.len());

    for project in projects.iter_mut() {
        debug!("Project: {}", project.name);
        if let Some(mut snapshot) = project.fetch_latest_snapshot(connection).await? {
            debug!("Snapshot: {} :: {}", snapshot.id, snapshot.components.len());

            // Fetch the alerts for the snapshot (previously stored)
            let mut alerts = Alerts::fetch_by_snapshot_id(connection, snapshot.id).await?;

            let results = GrypeDatabase::matcher(connection, grypedb, &mut snapshot).await?;

            // Find all the alerts that are not in results
            for alert in alerts.iter_mut() {
                if !results.iter().any(|r| r.id == alert.id) {
                    debug!("Marking Alert as Resolved: {}", alert.id);
                    alert.state = SecurityState::Secure;
                    alert.update(connection).await?;
                }
            }

            info!(
                "Project('{}', snapshot = '{}', components = '{}', vulnerabilities = '{}')",
                project.name,
                snapshot.id,
                snapshot.components.len(),
                results.len()
            );
        } else {
            warn!("No snapshots for project: {}", project.name);
        }
    }

    Ok(())
}

async fn reset_polling<'a, T>(connection: &'a T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    ServerSettings::fetch_by_name(connection, Setting::SecurityAdvisoriesPolling)
        .await?
        .set_update(connection, "disabled")
        .await?;

    Ok(())
}
