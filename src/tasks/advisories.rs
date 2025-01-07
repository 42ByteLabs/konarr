//! # Task - Advisories

use crate::{
    bom::{BomParser, Parsers},
    models::{security::SecurityState, Alerts, Projects, ServerSettings, Setting},
    tools::{Grype, Tool},
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
        let mut updated_last =
            ServerSettings::fetch_by_name(connection, Setting::SecurityAdvisoriesUpdated).await?;

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
        match GrypeDatabase::sync(&grype_path).await {
            Ok(new) => {
                if new {
                    info!("New Advisory Database Synced");

                    let mut grypedb_connection = GrypeDatabase::connect(&grype_path).await?;
                    grypedb_connection.fetch_vulnerabilities().await?;

                    info!("Scanning projects for security alerts");
                    scan_projects(config, connection).await?;
                    info!("Project scanning complete");
                } else {
                    info!("Advisory Database Synced but no new advisories, skipping project scanning for security alerts");
                }
            }
            Err(e) => {
                warn!("Advisory Sync Error: {}", e);
                reset_polling(connection).await?;
            }
        };

        updated_last
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

/// Scan for security alerts
pub async fn scan<'a, T>(config: &'a Config, connection: &'a T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    if ServerSettings::get_bool(connection, Setting::Security).await? {
        log::info!("Scanning projects for security alerts");

        scan_projects(config, connection).await?;
        Ok(())
    } else {
        Err(KonarrError::UnknownError(
            "Advisories Polling is disabled".to_string(),
        ))
    }
}

/// Scan every project for security alerts
pub async fn scan_projects<'a, T>(config: &'a Config, connection: &'a T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    info!("Scanning projects snapshots for security alerts");

    // Ensure Grype is installed and available
    let mut tool_grype = Grype::init().await;
    if !tool_grype.is_available() {
        warn!("Installing Grype, this may take a few minutes...");
        tool_grype.install().await?;
    }
    log::debug!("Grype Config: {:?}", tool_grype);

    let mut projects = Projects::fetch_all(connection).await?;
    info!("Projects Count: {}", projects.len());

    for project in projects.iter_mut() {
        debug!("Project: {}", project.name);
        if let Some(mut snapshot) = project.fetch_latest_snapshot(connection).await? {
            debug!("Snapshot: {} :: {}", snapshot.id, snapshot.components.len());
            snapshot.fetch_metadata(connection).await?;

            // Fetch the alerts for the snapshot (previously stored)
            let mut alerts = Alerts::fetch_by_snapshot_id(connection, snapshot.id).await?;

            if let Some(tool_alerts) = snapshot.find_metadata("security.tools.alerts") {
                if tool_alerts.as_bool() {
                    // If the `tool alerts` setting is disabled, we
                    if ServerSettings::get_bool(connection, Setting::SecurityToolsAlerts).await? {
                        info!(
                        "Project('{}', snapshot = '{}', components = '{}', vulnerabilities = '{}')",
                        project.name,
                        snapshot.id,
                        snapshot.components.len(),
                        alerts.len()
                    );
                        info!("Security Alerts coming from tools, skipping");
                        continue;
                    } else {
                        info!(
                            "Security Tools Alerts setting is disabled, scanning project for security alerts"
                        );
                    }
                }
            }

            let mut results = Vec::new();

            if let Some(bom_path) = snapshot.find_metadata("bom.path") {
                let full_path = config.sboms_path()?.join(bom_path.as_string());
                if !full_path.exists() {
                    warn!("SBOM does not exist: {}", full_path.display());
                    continue;
                }
                log::debug!("Using Grype to scan SBOM: {}", full_path.display());

                let bom = tool_grype.run(&full_path.display().to_string()).await?;
                let sbom = Parsers::parse(bom.as_bytes())?;
                log::debug!(
                    "BillOfMaterials(comps='{}', vulns='{}')",
                    sbom.components.len(),
                    sbom.vulnerabilities.len()
                );

                for vuln in sbom.vulnerabilities.iter() {
                    log::trace!("Vulnerability: {:?}", vuln);
                    let alts = Alerts::from_bom_vulnerability(connection, &snapshot, vuln).await?;
                    results.extend(alts);
                }
            } else {
                // TODO: Should we write the SBOM to disk?
                log::warn!(
                    "No SBOM path found for `{}`, skipping scanning of `{}`",
                    snapshot.id,
                    project.name,
                );
                // results = GrypeDatabase::matcher(connection, grypedb, &mut snapshot).await?;
                for alert in alerts.iter_mut() {
                    alert.close(connection).await?;
                }
            }

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
