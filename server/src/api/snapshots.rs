use geekorm::prelude::*;
use konarr::{
    models::{
        self, SnapshotMetadataKey,
        security::{Advisories, Alerts, SecuritySeverity},
    },
    tasks::{TaskTrait, sbom::SbomTask},
};
use rocket::{State, data::ToByteUnit, serde::json::Json};
use std::{collections::HashMap, str::FromStr};

use super::{
    ApiResponse, ApiResult,
    dependencies::DependencyResp,
    security::{AlertResp, SecuritySummary},
};
use crate::{
    AppState,
    error::KonarrServerError,
    guards::{Pagination, Session},
};

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
    /// Snapshot ID
    id: i32,
    /// Status of the snapshot
    status: Option<String>,
    /// If there was an error while processing the snapshot
    error: Option<String>,
    /// Where the snapshot was updated
    created_at: chrono::DateTime<chrono::Utc>,
    /// When the snapshot was Updated
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Dependency count
    dependencies: i32,
    /// Security Summary
    security: SecuritySummary,
    /// Metadata of the snapshot
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
                return Err(KonarrServerError::SnapshotNotFoundError(id as i32));
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

/// Create a new snapshot for a project
///
/// The project ID must be provided in the request body.
#[post("/", data = "<snapshot>")]
pub(crate) async fn create_snapshot(
    state: &State<AppState>,
    _session: Session,
    snapshot: Json<SnapshotCreateReq>,
) -> ApiResult<SnapshotResp> {
    log::info!("Creating snapshot for Project: {}", snapshot.project_id);

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
    log::debug!("Project: {:?}", project);

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

/// Update snapshot metadata
///
/// The metadata should be provided as a JSON object in the request body.
#[patch("/<id>/metadata", data = "<metadata>")]
pub(crate) async fn patch_snapshot_metadata(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    metadata: Json<HashMap<String, String>>,
) -> ApiResult<SnapshotResp> {
    log::info!("Updating metadata for snapshot: {}", id);

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

/// Upload a SBOM for a snapshot
///
/// The SBOM should be provided in the request body as raw data.
#[post("/<id>/bom", data = "<data>")]
pub(crate) async fn upload_bom(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    data: rocket::data::Data<'_>,
) -> ApiResult<SnapshotResp> {
    log::info!("Uploading SBOM for snapshot: {}", id);
    let mut snapshot =
        models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await?;

    let data = data
        .open(10.megabytes())
        .into_bytes()
        .await
        .map_err(|_| konarr::KonarrError::ParseSBOM("Failed to read data".to_string()))?;

    log::info!("Adding SBOM to snapshot: {}", snapshot.id);
    snapshot
        .add_bom(&state.connection().await, data.to_vec())
        .await?;

    SbomTask::sbom(snapshot.id.into())
        .spawn_task(&state.database)
        .await?;

    Ok(Json(snapshot.into()))
}

/// Get dependencies for a snapshot
///
/// Optionally, a search query can be provided to filter dependencies by name
/// or version.
#[get("/<id>/dependencies?<search>")]
pub(crate) async fn get_snapshot_dependencies(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    search: Option<String>,
    pagination: Pagination,
) -> ApiResult<ApiResponse<Vec<DependencyResp>>> {
    log::info!("Fetching dependencies for snapshot: {}", id);

    let mut query = models::Dependencies::query_select()
        .where_eq("snapshot_id", id as i32)
        .order_by("name", QueryOrder::Asc);
    let mut query_count = models::Dependencies::query_count().where_eq("snapshot_id", id as i32);

    if let Some(search) = &search {
        log::info!("Searching dependencies for: '{}'", search);
        query = query.and().where_like("name", format!("%{}%", search));
        query_count = query_count
            .and()
            .where_like("name", format!("%{}%", search));
    }

    let total =
        models::Dependencies::row_count(&state.connection().await, query_count.build()?).await?;

    let page = pagination.page_with_total(total as u32);

    let mut snapshot =
        models::Snapshot::query_first(&state.connection().await, query.page(&page).build()?)
            .await?;
    snapshot.fetch_metadata(&state.connection().await).await?;

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
        total as u32,
        page.pages(),
    )))
}

/// Get alerts for a snapshot
///
/// By default, 'Unknown' severity alerts are filtered out
#[get("/<id>/alerts?<search>&<severity>")]
pub(crate) async fn get_snapshot_alerts(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    search: Option<String>,
    severity: Option<String>,
    pagination: Pagination,
) -> ApiResult<ApiResponse<Vec<AlertResp>>> {
    log::info!("Fetching alerts for snapshot: {}", id);

    let snapshot =
        models::Snapshot::fetch_by_primary_key(&state.connection().await, id as i32).await?;

    let mut query_count = Alerts::query_count()
        .join(Advisories::table())
        .where_eq("snapshot_id", snapshot.id);
    let mut query = Alerts::query_select()
        .join(Advisories::table())
        .where_eq("snapshot_id", snapshot.id)
        .order_by("Alerts.created_at", QueryOrder::Asc);

    if let Some(search) = search {
        log::info!("Searching alerts for: '{}'", search);
        query = query
            .and()
            .where_like("Advisories.advisory_id", format!("%{}%", search));
        query_count = query_count
            .and()
            .where_like("Advisories.advisory_id", format!("%{}%", search));
    }
    if let Some(severity) = severity {
        let severity = SecuritySeverity::from(severity);

        log::info!("Filtering alerts by severity: {:?}", severity);

        query = query.and().where_eq("Advisories.severity", &severity);
        query_count = query_count.and().where_eq("Advisories.severity", severity);
    } else {
        // By default, filter out 'Unknown' severity alerts
        query = query
            .and()
            .where_ne("Advisories.severity", SecuritySeverity::Unknown);
    }

    let total = Alerts::row_count(&state.connection().await, query_count.build()?).await?;
    let page = pagination.page_with_total(total as u32);

    let mut alerts = Alerts::query(&state.connection().await, query.page(&page).build()?).await?;
    for alert in alerts.iter_mut() {
        alert.fetch(&state.connection().await).await?;
    }

    info!(
        "Found `{}` alerts in snapshot `{}`",
        alerts.len(),
        snapshot.id
    );
    let count = alerts.len() as u32;

    Ok(Json(ApiResponse::new(
        alerts.into_iter().map(|a| a.into()).collect(),
        total as u32,
        count,
        page.pages(),
    )))
}

/// Get all snapshots with pagination
#[get("/")]
pub async fn get_snapshots(
    state: &State<AppState>,
    _session: Session,
    pagination: Pagination,
) -> ApiResult<Vec<SnapshotResp>> {
    let page = pagination.page();

    let mut snapshots = models::Snapshot::query(
        &state.connection().await,
        models::Snapshot::query_select().page(&page).build()?,
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
            error: snapshot.error.clone(),
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
            error: snapshot.error,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            dependencies: count,
            security,
            metadata,
        }
    }
}
