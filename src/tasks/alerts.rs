//! Security Alerts Tasks

use std::collections::HashMap;

use crate::models::{
    dependencies::snapshots::AlertsSummary, security::SecuritySeverity, settings::Setting,
    ProjectType, Projects, ServerSettings,
};
use geekorm::prelude::*;
use log::{debug, info};

/// Alert Calculator Task
pub async fn alert_calculator<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    if !ServerSettings::feature_security(connection).await? {
        log::error!("Security Feature is not enabled");
        return Ok(());
    }
    info!("Task - Running Alert Calculator");

    let mut summary = AlertsSummary::new();
    let mut total = 0;

    let page = Page::from((0, 1_000));
    let mut projects =
        Projects::fetch_project_type(connection, ProjectType::Container, &page).await?;
    log::debug!("Found `{}` Container projects", projects.len());

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
        log::warn!("Total Alert Count Mismatch: {} != {}", total_check, total);
    }

    debug!("Global Alerts Updated");

    Ok(())
}

/// Calculate Group Alerts
pub async fn calculate_group_alerts<'a, T>(
    connection: &'a T,
    projects: &Vec<Projects>,
    project_summaries: &HashMap<i32, AlertsSummary>,
) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        bom::{
            sbom::{BomComponent, BomType},
            BillOfMaterials,
        },
        models::*,
        utils::config::DatabaseConfig,
    };

    async fn database() -> Result<Arc<Mutex<libsql::Connection>>, crate::KonarrError> {
        let config = DatabaseConfig {
            path: Some(":memory:".to_string()),
            ..Default::default()
        };
        let conn = DatabaseConfig::connection(&config).await?;
        let connection = Arc::new(Mutex::new(conn));
        crate::models::database_create(&connection).await?;

        let mut bill = BillOfMaterials::new(BomType::CycloneDX_1_6, "0.1.0".to_string());
        bill.components = vec![
            //
            BomComponent::from_purl("pkg:deb/debian/curl@7.68.0".to_string()),
            BomComponent::from_purl("pkg:deb/debian/apt@1".to_string()),
        ];

        for project_id in 1..99 {
            let mut project = Projects::new(format!("test-{}", project_id), ProjectType::Container);
            project.save(&connection).await?;
            assert_eq!(project.id, project_id.into());

            let mut snapshot = Snapshot::new();
            snapshot.add_bom(&connection, &bill).await?;
            snapshot.fetch_or_create(&connection).await?;

            project.add_snapshot(&connection, snapshot).await?;
            project.update(&connection).await?;
        }

        let total = Projects::total(&connection).await?;
        assert_eq!(total, 100);

        Ok(connection)
    }

    #[tokio::test]
    async fn test_alert_calculator() -> Result<(), crate::KonarrError> {
        let connection = database().await?;

        alert_calculator(&connection).await?;

        Ok(())
    }
}
