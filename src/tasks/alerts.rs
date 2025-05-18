//! Security Alerts Tasks

use std::collections::HashMap;

use crate::models::security::SecurityState;
use crate::models::{
    Alerts, Projects, ServerSettings, dependencies::snapshots::AlertsSummary,
    security::SecuritySeverity, settings::Setting,
};
use async_trait::async_trait;
use geekorm::{ConnectionManager, prelude::*};
use log::{debug, info};

use super::TaskTrait;

/// Alert Calculator Task
#[derive(Default)]
pub struct AlertCalculatorTask;

#[async_trait]
impl TaskTrait for AlertCalculatorTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        if !ServerSettings::feature_security(&database.acquire().await).await? {
            log::error!("Security Feature is not enabled");
            return Ok(());
        }

        let mut projects = Projects::fetch_containers(&database.acquire().await).await?;

        for project in projects.iter_mut() {
            project
                .fetch_latest_snapshot_alerts(&database.acquire().await)
                .await?;

            // This is needed to ensure all the alerts are correct
            // TODO: This should be removed in the future
            project
                .check_latest_snapshot(&database.acquire().await)
                .await?;
        }

        log::debug!("Found `{}` Container projects", projects.len());

        alert_calculator(&database.acquire().await, &mut projects).await?;

        alerts_cleanup(&database, &projects).await?;

        Ok(())
    }
}

/// Alert Calculator Task
pub async fn alert_calculator(
    connection: &geekorm::Connection<'_>,
    projects: &mut Vec<Projects>,
) -> Result<(), crate::KonarrError> {
    info!("Task - Running Alert Calculator");

    let mut summary = AlertsSummary::new();
    let mut total = 0;

    let mut project_summaries: HashMap<i32, AlertsSummary> = HashMap::new();

    for project in projects.iter_mut() {
        if let Some(snapshot) = project.snapshots.last_mut() {
            info!(
                "Project('{}', snapshot='{}', alerts='{}')",
                project.name,
                snapshot.id,
                snapshot.alerts.len()
            );

            let snap_summary = snapshot.calculate_alerts_summary(connection).await?;
            for (key, value) in snap_summary.iter() {
                *summary.entry(key.clone()).or_insert(0) += value;
                total += value;
            }

            project_summaries.insert(project.id.into(), snap_summary);
        }
    }

    calculate_group_alerts(connection, projects, &project_summaries).await?;

    debug!("Calculating Global Alerts Summary");
    debug!("Global Summary: {:?}", summary);

    let mut global_alerts = ServerSettings::get_namespace(connection, "security.alerts").await?;
    let mut total_check = 0;

    for galert in global_alerts.iter_mut() {
        if galert.name == Setting::SecurityAlertsTotal {
            galert.value = total.to_string();
            galert.update(connection).await?;
            continue;
        }

        let severity = SecuritySeverity::from(galert.name.to_string());

        if let Some(value) = summary.get(&severity) {
            galert.value = value.to_string();
            total_check += value;
        } else {
            galert.value = "0".to_string();
        }

        galert.update(connection).await?;
    }

    if total_check != total {
        log::warn!("Total Alert Count Mismatch: {} != {}", total_check, total);
    }

    debug!("Global Alerts Updated");

    Ok(())
}

/// Calculate Group Alerts
pub async fn calculate_group_alerts(
    connection: &geekorm::Connection<'_>,
    projects: &Vec<Projects>,
    project_summaries: &HashMap<i32, AlertsSummary>,
) -> Result<(), crate::KonarrError> {
    log::debug!("Calculating Group Alerts");
    let mut groups = Projects::fetch_servers(connection).await?;
    log::debug!("Found {} groups", groups.len());

    for group in groups.iter_mut() {
        let group_id: i32 = group.id.into();

        if let Some(snapshot) = group.snapshots.last_mut() {
            log::debug!("Group('{}', snapshot='{}')", group.name, snapshot.id);

            let mut group_summary = AlertsSummary::new();
            let mut group_total = 0;

            let children: Vec<AlertsSummary> = projects
                .iter()
                .filter(|p| p.parent == group_id)
                .filter_map(|c| project_summaries.get(&c.id.into()))
                .cloned()
                .collect();

            for child in children.iter() {
                for (key, value) in child.iter() {
                    *group_summary.entry(key.clone()).or_insert(0) += value;
                    group_total += value;
                }
            }

            snapshot
                .calculate_alerts(connection, &group_summary)
                .await?;
        }
    }
    Ok(())
}

/// Alerts Cleanup Task
///
/// Any alerts in old snapshots that are not referenced by any project
/// will be marked as Secure / Closed
pub async fn alerts_cleanup(
    database: &ConnectionManager,
    projects: &Vec<Projects>,
) -> Result<(), crate::KonarrError> {
    log::info!("Task - Running Alerts Cleanup");

    // Fetch all active alerts
    let alerts =
        Alerts::fetch_by_state(&database.acquire().await, SecurityState::Vulnerable).await?;
    log::info!("Found {} active alerts", alerts.len());

    let snapshot_alerts: Vec<Alerts> = projects
        .iter()
        .filter_map(|project| project.snapshots.last())
        .map(|snapshot| snapshot.alerts.clone())
        .flatten()
        .collect();

    log::info!("Found {} active alerts in snapshots", snapshot_alerts.len());

    // Filter alerts that are not in any active snapshot
    let mut orphaned_alerts: Vec<Alerts> = alerts
        .iter()
        .filter(|alert| {
            !snapshot_alerts
                .iter()
                .any(|snapshot_alert| snapshot_alert.id == alert.id)
        })
        .cloned()
        .collect();
    log::info!("Found {} orphaned alerts", orphaned_alerts.len());

    assert_eq!(orphaned_alerts.len(), alerts.len() - snapshot_alerts.len());

    if !orphaned_alerts.is_empty() {
        log::info!("Found {} active alerts", alerts.len());
        log::info!("Found {} orphaned alerts", orphaned_alerts.len());

        for alert in orphaned_alerts.iter_mut() {
            alert.state = SecurityState::Secure;
            alert.update(&database.acquire().await).await?;
        }
    } else {
        log::debug!("No orphaned alerts found");
    }

    Ok(())
}
