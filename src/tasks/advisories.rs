//! # Task - Advisories

use crate::{
    models::{Projects, ServerSettings, Setting},
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

                    let grypedb_connection = GrypeDatabase::connect(&grype_path).await?;

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

    let grype_conn: libsql::Connection = match GrypeDatabase::connect(&grype_path).await {
        Ok(conn) => conn,
        Err(_) => {
            return Ok(());
        }
    };

    // Set Version
    let grype_id = match GrypeDatabase::fetch_grype(&grype_conn).await {
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
    grypedb_connection: &libsql::Connection,
) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    info!("Scanning projects snapshots for security alerts");

    let mut projects = Projects::fetch_all(connection).await?;
    debug!("Projects Count: {}", projects.len());

    for project in projects.iter_mut() {
        debug!("Project: {}", project.name);
        if let Some(mut snapshot) = project.fetch_latest_snapshot(connection).await? {
            debug!("Snapshot: {} :: {}", snapshot.id, snapshot.components.len());

            let results = snapshot
                .scan_with_grype(connection, grypedb_connection)
                .await?;
            debug!("Vulnerabilities: {}", results.len());
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
