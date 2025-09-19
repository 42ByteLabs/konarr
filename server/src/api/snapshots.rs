use geekorm::prelude::*;
use konarr::{
    models::{
        self, SnapshotMetadataKey,
        security::{Advisories, Alerts, SecuritySeverity},
    },
    tasks::{TaskTrait, sbom::SbomTask},
};
use log::{debug, info};
use rocket::{State, data::ToByteUnit, serde::json::Json};
use std::{collections::HashMap, str::FromStr};

use super::{
    ApiResponse, ApiResult,
    dependencies::DependencyResp,
    security::{AlertResp, SecuritySummary},
};
use crate::{AppState, error::KonarrServerError, guards::Session};

pub fn routes() -> Vec<rocket::Route> {
    routes![
        get_snapshot,
        get_snapshots,
        get_snapshot_dependencies,
        get_snapshot_alerts,
        create_snapshot,
        upload_bom,
        patch_snapshot_metadata,
    ]
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub(crate) struct SnapshotResp {
    id: i32,

    created_at: chrono::DateTime<chrono::Utc>,

    status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,

    dependencies: i32,

    security: SecuritySummary,

    metadata: HashMap<String, String>,
}

/// Get a snapshot by ID
#[get("/<id>")]
pub(crate) async fn get_snapshot(
    state: &State<AppState>,
    _session: Session,
    id: u32,
) -> ApiResult<SnapshotResp> {
    info!("Fetching snapshot: {}", id);
    let mut snapshot =
        match models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                log::error!("Failed to fetch snapshot: {:?}", e);
                return Err(KonarrServerError::SnapshotNotFoundError(id as i32).into());
            }
        };
    snapshot.fetch_metadata(&state.connection().await).await?;

    Ok(Json(snapshot.into()))
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SnapshotCreateReq {
    project_id: u32,
}

#[post("/", data = "<snapshot>")]
pub(crate) async fn create_snapshot(
    state: &State<AppState>,
    _session: Session,
    snapshot: Json<SnapshotCreateReq>,
) -> ApiResult<SnapshotResp> {
    info!("Creating snapshot for Project: {}", snapshot.project_id);

    let mut project = match models::Projects::fetch_by_primary_key(
        &state.connection().await,
        snapshot.project_id as i32,
    )
    .await
    {
        Ok(project) => project,
        Err(geekorm::Error::NoRowsFound { query: _ }) => {
            log::error!("Project not found: {}", snapshot.project_id);
            return Err(KonarrServerError::ProjectNotFoundError(
                snapshot.project_id as i32,
            ));
        }
        Err(e) => {
            log::error!("Failed to fetch project: {:?}", e);
            return Err(KonarrServerError::InternalServerError);
        }
    };
    debug!("Project: {:?}", project);

    let mut snapshot = models::Snapshot::new();
    match snapshot.save(&state.connection().await).await {
        Ok(snapshot) => snapshot,
        Err(e) => {
            log::error!("Failed to create snapshot: {:?}", e);
            return Err(KonarrServerError::InternalServerError);
        }
    };
    project
        .add_snapshot(&state.connection().await, snapshot.clone())
        .await?;

    Ok(Json(snapshot.into()))
}

#[patch("/<id>/metadata", data = "<metadata>")]
pub(crate) async fn patch_snapshot_metadata(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    metadata: Json<HashMap<String, String>>,
) -> ApiResult<SnapshotResp> {
    info!("Updating metadata for snapshot: {}", id);
    let mut snapshot =
        match models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await {
            Ok(snapshot) => snapshot,
            Err(e) => {
                log::error!("Failed to fetch snapshot: {:?}", e);
                return Err(KonarrServerError::SnapshotNotFoundError(id as i32));
            }
        };
    snapshot.fetch_metadata(&state.connection().await).await?;

    for (key, value) in metadata.iter() {
        if value.is_empty() {
            continue;
        }
        let metadata_key = match SnapshotMetadataKey::from_str(key) {
            Ok(key) => key,
            Err(e) => {
                log::error!("Invalid metadata key: {}", e);
                return Err(konarr::KonarrError::InvalidData(format!(
                    "Invalid metadata key: {}",
                    e
                ))
                .into());
            }
        };

        log::info!("Setting metadata: {} = {}", metadata_key, value);

        snapshot
            .set_metadata(&state.connection().await, metadata_key, value)
            .await?;
    }

    Ok(Json(snapshot.into()))
}

#[post("/<id>/bom", data = "<data>")]
pub(crate) async fn upload_bom(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    data: rocket::data::Data<'_>,
) -> ApiResult<SnapshotResp> {
    info!("Uploading SBOM for snapshot: {}", id);
    let mut snapshot =
        models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await?;

    let data = data
        .open(10.megabytes())
        .into_bytes()
        .await
        .map_err(|_| konarr::KonarrError::ParseSBOM("Failed to read data".to_string()))?;

    info!("Adding SBOM to snapshot: {}", snapshot.id);
    snapshot
        .add_bom(&state.connection().await, data.to_vec())
        .await?;

    SbomTask::sbom(snapshot.id.into())
        .spawn_task(&state.database)
        .await?;

    Ok(Json(snapshot.into()))
}

#[get("/<id>/dependencies?<search>&<page>&<limit>")]
pub(crate) async fn get_snapshot_dependencies(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    search: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<ApiResponse<Vec<DependencyResp>>> {
    let page = Page::from((page, limit));

    let mut snapshot =
        models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await?;
    snapshot.fetch_metadata(&state.connection().await).await?;

    let total = snapshot.find_metadata_usize("bom.dependencies.count");

    let mut deps = if let Some(search) = search {
        models::Dependencies::search(&state.connection().await, snapshot.id, search).await?
    } else {
        snapshot
            .fetch_dependencies(&state.connection().await, &page)
            .await?
    };

    for dep in &mut deps {
        dep.fetch(&state.connection().await).await?;
    }

    Ok(Json(ApiResponse::new(
        deps.into_iter().map(|d| d.into()).collect(),
        total as u32,
        page.pages(),
    )))
}

#[get("/<id>/alerts?<search>&<severity>&<page>&<limit>")]
pub(crate) async fn get_snapshot_alerts(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    search: Option<String>,
    severity: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<ApiResponse<Vec<AlertResp>>> {
    let snapshot =
        models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await?;
    let total = snapshot
        .fetch_alerts_count(&state.connection().await)
        .await?;

    let page = Page::from((page, limit));

    let alerts: Vec<Alerts> = if let Some(_search) = search {
        vec![] // TODO: Implement search
    } else if let Some(severity) = severity {
        let severity = SecuritySeverity::from(severity);

        info!("Filtering alerts by severity: {:?}", severity);
        let mut alerts = Alerts::query(
            &state.connection().await,
            Alerts::query_select()
                .join(Advisories::table())
                .where_eq("snapshot_id", snapshot.id)
                .and()
                .where_eq("Advisories.severity", severity)
                .page(&page)
                .build()?,
        )
        .await?;
        for alert in alerts.iter_mut() {
            alert.fetch(&state.connection().await).await?;
        }
        alerts
    } else {
        snapshot
            .fetch_alerts_page(&state.connection().await, &page)
            .await?
    };
    info!(
        "Found `{}` alerts in snapshot `{}`",
        alerts.len(),
        snapshot.id
    );

    Ok(Json(ApiResponse::new(
        alerts.into_iter().map(|a| a.into()).collect(),
        total as u32,
        page.pages(),
    )))
}

#[get("/?<page>&<limit>")]
pub async fn get_snapshots(
    state: &State<AppState>,
    _session: Session,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<Vec<SnapshotResp>> {
    let page = page.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(25) as usize;

    let mut snapshots = models::Snapshot::query(
        &state.connection().await,
        models::Snapshot::query_select()
            .limit(limit)
            .offset(page * limit)
            .build()?,
    )
    .await?;

    let mut resp = Vec::new();

    for snapshot in snapshots.iter_mut() {
        snapshot.fetch_metadata(&state.connection().await).await?;

        let mut count = 0;

        let mut metadata = HashMap::new();
        for (name, meta) in snapshot.metadata.iter() {
            if *name == SnapshotMetadataKey::DependenciesTotal {
                count = meta.as_string().parse().unwrap_or(0);
                continue;
            }
            metadata.insert(name.to_string(), meta.as_string());
        }

        resp.push(SnapshotResp {
            id: snapshot.id.into(),
            status: Some(snapshot.state.to_string()),
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            dependencies: count,
            security: SecuritySummary::default(),
            metadata,
        });
    }

    Ok(Json(resp))
}

impl From<models::Snapshot> for SnapshotResp {
    fn from(snapshot: models::Snapshot) -> Self {
        let mut count = 0;
        let mut metadata = HashMap::new();

        for (name, meta) in snapshot.metadata.iter() {
            if *name == SnapshotMetadataKey::DependenciesTotal {
                // count = meta.as_string().parse().unwrap_or(0);
                count = meta.as_i32();
                continue;
            } else if name.to_string().starts_with("security.") {
                continue;
            }
            metadata.insert(name.to_string(), meta.as_string());
        }

        let security = SecuritySummary::from(&snapshot);

        SnapshotResp {
            id: snapshot.id.into(),
            status: Some(snapshot.state.to_string()),
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            dependencies: count,
            security,
            metadata,
        }
    }
}
