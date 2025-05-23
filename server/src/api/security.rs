//! # Security API

use geekorm::{GeekConnector, QueryBuilderTrait, QueryOrder, prelude::Page};
use konarr::models::{
    Snapshot,
    security::{Alerts, SecuritySeverity, SecurityState},
};
use log::info;
use rocket::{State, serde::json::Json};

use super::{ApiResponse, ApiResult, dependencies::DependencyResp};
use crate::{AppState, guards::Session};

/// Security Summary
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct SecuritySummary {
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub informational: u32,
    pub unmaintained: u32,
    pub malware: u32,
    pub unknown: u32,
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_alerts, get_alert]
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub(crate) struct AlertResp {
    id: i32,
    name: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dependency: Option<DependencyResp>,
}

#[get("/?<page>&<limit>&<search>&<state>&<severity>")]
pub(crate) async fn get_alerts(
    app_state: &State<AppState>,
    _session: Session,
    page: Option<u32>,
    limit: Option<u32>,
    state: Option<String>,
    search: Option<String>,
    severity: Option<String>,
) -> ApiResult<ApiResponse<Vec<AlertResp>>> {
    let total = Alerts::count_vulnerable(&app_state.connection().await).await?;
    let mut page = Page::from((page, limit));
    page.set_total(total as u32);

    let state = SecurityState::from(state);

    let alerts = if let Some(search) = search {
        info!("Searching for alerts: {}", search);
        Alerts::search(&app_state.connection().await, search).await?
    } else if let Some(severity) = severity {
        let severity = SecuritySeverity::from(severity);
        info!("Filtering alerts by severity: {:?}", severity);
        // Alerts::filter_page(connection, vec![("severity", severity)], &page).await?
        Alerts::filter_severity(&app_state.connection().await, severity, &page).await?
    } else {
        info!("Getting alerts");
        Alerts::query(
            &app_state.connection().await,
            Alerts::query_select()
                .where_eq("state", state)
                .order_by("id", QueryOrder::Asc)
                .page(&page)
                .build()?,
        )
        .await?
    };

    Ok(Json(ApiResponse::new(
        alerts.into_iter().map(|a| a.into()).collect(),
        total,
        page.pages(),
    )))
}

#[get("/<id>")]
pub(crate) async fn get_alert(
    state: &State<AppState>,
    _session: Session,
    id: i32,
) -> ApiResult<AlertResp> {
    let connection = state.connection().await;
    let mut alert = Alerts::fetch_by_primary_key(&connection, id).await?;

    alert.fetch_advisory_id(&connection).await?;
    alert.fetch_metadata(&connection).await?;

    alert.fetch_snapshot_id(&connection).await?;

    // Fetch the dependency
    alert.fetch_dependency_id(&connection).await?;
    alert.dependency_id.data.fetch(&connection).await?;

    info!(
        "Fetched alert: {} (dep: {})",
        alert.name, alert.dependency_id
    );

    Ok(Json(alert.into()))
}

impl From<Alerts> for AlertResp {
    fn from(value: Alerts) -> Self {
        let severity = value.advisory_id.data.severity.to_string();

        let dependency: DependencyResp = value.dependency_id.clone().data.into();

        Self {
            id: value.id.into(),
            name: value.name.clone(),
            severity,
            description: value.description(),
            url: value.url(),
            dependency: Some(dependency),
            ..Default::default()
        }
    }
}

impl From<&Snapshot> for SecuritySummary {
    fn from(snapshot: &Snapshot) -> Self {
        let total = snapshot.find_metadata_usize("security.alerts.total") as u32;
        let critical = snapshot.find_metadata_usize("security.alerts.critical") as u32;
        let high = snapshot.find_metadata_usize("security.alerts.high") as u32;
        let medium = snapshot.find_metadata_usize("security.alerts.medium") as u32;
        let low = snapshot.find_metadata_usize("security.alerts.low") as u32;
        let informational = snapshot.find_metadata_usize("security.alerts.informational") as u32;
        let unmaintained = snapshot.find_metadata_usize("security.alerts.unmaintained") as u32;
        let malware = snapshot.find_metadata_usize("security.alerts.malware") as u32;
        let unknown = snapshot.find_metadata_usize("security.alerts.unknown") as u32;

        Self {
            total,
            critical,
            high,
            medium,
            low,
            informational,
            unmaintained,
            malware,
            unknown,
        }
    }
}
