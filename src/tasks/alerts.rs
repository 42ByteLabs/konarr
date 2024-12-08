//! Security Alerts Tasks

use crate::models::{
    dependencies::snapshots::AlertsSummary, security::SecuritySeverity, settings::Setting,
    Projects, ServerSettings,
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
    info!("Running Alert Calculator Task");

    let mut projects = Projects::fetch_all(connection).await?;
    let mut summary = AlertsSummary::new();
    let mut total = 0;

    for project in projects.iter_mut() {
        if let Some(mut snapshot) = project.fetch_latest_snapshot(connection).await? {
            debug!("Project('{}', snapshot='{}')", project.name, snapshot.id);

            let snap_summary = snapshot.calculate_alerts_summary(connection).await?;
            for (key, value) in snap_summary.iter() {
                *summary.entry(key.clone()).or_insert(0) += value;
                total += value;
            }
        }
    }

    info!("Calculating Global Alerts Summary");
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

    info!("Global Alerts Updated");

    Ok(())
}
