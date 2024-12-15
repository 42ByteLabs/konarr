//! Security Alerts Tasks

use std::collections::HashMap;

use crate::models::{
    dependencies::snapshots::AlertsSummary, security::SecuritySeverity, settings::Setting,
    ProjectType, Projects, ServerSettings,
};
use geekorm::prelude::*;
use log::{debug, info};

/// Alert Calculator Task
pub async fn alert_calculator<T>(connection: &T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
{
    if !ServerSettings::feature_security(connection).await? {
        log::error!("Security Feature is not enabled");
        return Ok(());
    }
    info!("Task - Running Alert Calculator");

    let mut summary = AlertsSummary::new();
    let mut total = 0;

    let mut projects =
        Projects::fetch_project_type(connection, ProjectType::Container, 1_000, 0).await?;
    let mut project_summaries: HashMap<i32, AlertsSummary> = HashMap::new();

    for project in projects.iter_mut() {
        if let Some(mut snapshot) = project.fetch_latest_snapshot(connection).await? {
            debug!("Project('{}', snapshot='{}')", project.name, snapshot.id);

            let snap_summary = snapshot.calculate_alerts_summary(connection).await?;
            for (key, value) in snap_summary.iter() {
                *summary.entry(key.clone()).or_insert(0) += value;
                total += value;
            }

            project_summaries.insert(project.id.into(), snap_summary);
        }
    }

    calculate_group_alerts(connection, &projects, &project_summaries).await?;

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
        log::error!("Total Alert Count Mismatch: {} != {}", total_check, total);
    }

    debug!("Global Alerts Updated");

    Ok(())
}

/// Calculate Group Alerts
pub async fn calculate_group_alerts<T>(
    connection: &T,
    projects: &Vec<Projects>,
    project_summaries: &HashMap<i32, AlertsSummary>,
) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'static,
{
    log::debug!("Calculating Group Alerts");
    // TODO: Only Server's are supported
    let mut groups = Projects::query(
        connection,
        Projects::query_select()
            .where_eq("project_type", ProjectType::Server)
            .build()?,
    )
    .await?;
    log::debug!("Found {} groups", groups.len());

    for group in groups.iter_mut() {
        let group_id: i32 = group.id.into();

        if let Some(mut snapshot) = group.fetch_latest_snapshot(connection).await? {
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
