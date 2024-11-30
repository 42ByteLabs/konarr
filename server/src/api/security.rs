//! # Security API

use geekorm::{prelude::Pagination, GeekConnector, QueryBuilderTrait, QueryOrder};
use konarr::models::{
    security::{Alerts, SecuritySeverity, SecurityState},
    Snapshot,
};
use log::info;
use rocket::{serde::json::Json, State};

use crate::{guards::Session, AppState};

use super::{ApiResponse, ApiResult};

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
    let connection = app_state.db.connect()?;

    let page = Pagination::from((page, limit));

    let total = Alerts::count_vulnerable(&connection).await?;
    let pages = (total as f32 / page.limit() as f32).ceil() as u32;

    let state = SecurityState::from(state);

    let alerts = if let Some(search) = search {
        info!("Searching for alerts: {}", search);
        Alerts::search(&connection, search).await?
    } else if let Some(severity) = severity {
        let severity = SecuritySeverity::from(severity);
        info!("Filtering alerts by severity: {:?}", severity);
        Alerts::filter_severity(&connection, severity, &page).await?
    } else {
        info!("Getting alerts");
        Alerts::query(
            &connection,
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
        pages,
    )))
}

#[get("/<id>")]
pub(crate) async fn get_alert(
    state: &State<AppState>,
    _session: Session,
    id: i32,
) -> ApiResult<AlertResp> {
    let connection = state.db.connect()?;

    let mut alert = Alerts::fetch_by_primary_key(&connection, id).await?;
    alert.fetch(&connection).await?;

    Ok(Json(alert.into()))
}

impl From<Alerts> for AlertResp {
    fn from(value: Alerts) -> Self {
        let severity = value.advisory_id.data.severity.to_string();
        Self {
            id: value.id.into(),
            name: value.name.clone(),
            severity,
        }
    }
}

impl From<&Snapshot> for SecuritySummary {
    fn from(snapshot: &Snapshot) -> Self {
        let total = snapshot.find_metadata_usize("security.counts.total") as u32;
        let critical = snapshot.find_metadata_usize("security.counts.critical") as u32;
        let high = snapshot.find_metadata_usize("security.counts.high") as u32;
        let medium = snapshot.find_metadata_usize("security.counts.medium") as u32;
        let low = snapshot.find_metadata_usize("security.counts.low") as u32;
        let informational = snapshot.find_metadata_usize("security.counts.informational") as u32;
        let unmaintained = snapshot.find_metadata_usize("security.counts.unmaintained") as u32;
        let malware = snapshot.find_metadata_usize("security.counts.malware") as u32;
        let unknown = snapshot.find_metadata_usize("security.counts.unknown") as u32;

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
