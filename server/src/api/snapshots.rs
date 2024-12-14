use geekorm::prelude::*;
use konarr::{
    bom::{BomParser, Parsers},
    models::{
        self,
        dependencies::snapshots::metadata::SnapshotMetadataKey,
        security::{Advisories, Alerts, SecuritySeverity},
    },
};
use log::info;
use rocket::{data::ToByteUnit, serde::json::Json, State};
use std::{collections::HashMap, str::FromStr};

use super::{
    dependencies::DependencyResp,
    security::{AlertResp, SecuritySummary},
    ApiResponse, ApiResult,
};
use crate::{guards::Session, AppState};

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
    let connection = state.db.connect()?;

    let mut snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;
    snapshot.fetch_metadata(&connection).await?;

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
    let connection = state.db.connect()?;

    let mut project =
        models::Projects::fetch_by_primary_key(&connection, snapshot.project_id as i32).await?;
    info!("Creating snapshot for Project: {}", project.id);

    let snapshot = models::Snapshot::create(&connection).await?;
    project.add_snapshot(&connection, snapshot.clone()).await?;

    Ok(Json(snapshot.into()))
}

#[patch("/<id>/metadata", data = "<metadata>")]
pub(crate) async fn patch_snapshot_metadata(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    metadata: Json<HashMap<String, String>>,
) -> ApiResult<SnapshotResp> {
    let connection = state.db.connect()?;

    let mut snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;
    snapshot.fetch_metadata(&connection).await?;

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

        snapshot
            .set_metadata(&connection, metadata_key, value)
            .await?;
    }

    // Run the statistics task in the background
    tokio::spawn(async move {
        konarr::tasks::statistics(&connection)
            .await
            .map_err(|e| {
                log::error!("Failed to run alert calculator: {:?}", e);
            })
            .ok();
    });

    Ok(Json(snapshot.into()))
}

#[post("/<id>/bom", data = "<data>")]
pub(crate) async fn upload_bom(
    state: &State<AppState>,
    _session: Session,
    id: u32,
    data: rocket::data::Data<'_>,
) -> ApiResult<SnapshotResp> {
    let connection = state.db.connect()?;

    let mut snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;

    // TODO: Implement file upload
    let data = data
        .open(10.megabytes())
        .into_bytes()
        .await
        .map_err(|_| konarr::KonarrError::ParseSBOM("Failed to read data".to_string()))?;

    info!("Read SBOM data: {} bytes", data.len());
    let bom = Parsers::parse(&data)?;
    debug!("Parsed SBOM: {:?}", bom);

    info!("Adding SBOM to snapshot: {}", snapshot.id);
    snapshot.add_bom(&connection, &bom).await?;

    tokio::spawn(async move {
        info!("Running statistics and alert calculator tasks");
        konarr::tasks::statistics(&connection)
            .await
            .map_err(|e| {
                log::error!("Failed to run statistics: {:?}", e);
            })
            .ok();
        konarr::tasks::alerts::alert_calculator(&connection)
            .await
            .map_err(|e| {
                log::error!("Failed to run alert calculator: {:?}", e);
            })
            .ok();
    });

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
    let connection = state.db.connect()?;

    let page = page.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(10) as usize;

    let mut snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;
    snapshot.fetch_metadata(&connection).await?;

    let total = snapshot.find_metadata_usize("bom.dependencies.count");

    let mut deps = if let Some(search) = search {
        models::Dependencies::search(&connection, snapshot.id, search).await?
    } else {
        snapshot
            .fetch_dependencies(&connection, page, limit)
            .await?
    };

    for dep in &mut deps {
        dep.fetch(&connection).await?;
    }

    Ok(Json(ApiResponse::new(
        deps.into_iter().map(|d| d.into()).collect(),
        total as u32,
        (total / limit) as u32,
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
    let connection = state.db.connect()?;

    let snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;
    let total = snapshot.fetch_alerts_count(&connection).await?;

    let page = Pagination::from((page, limit));
    let pages = (total as f32 / page.limit() as f32).ceil() as u32;

    let alerts: Vec<Alerts> = if let Some(_search) = search {
        vec![] // TODO: Implement search
    } else if let Some(severity) = severity {
        let severity = SecuritySeverity::from(severity);

        info!("Filtering alerts by severity: {:?}", severity);
        let mut alerts = Alerts::query(
            &connection,
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
            alert.fetch(&connection).await?;
        }
        alerts
    } else {
        snapshot.fetch_alerts_page(&connection, &page).await?
    };
    info!(
        "Found `{}` alerts in snapshot `{}`",
        alerts.len(),
        snapshot.id
    );

    Ok(Json(ApiResponse::new(
        alerts.into_iter().map(|a| a.into()).collect(),
        total as u32,
        pages,
    )))
}

#[get("/?<page>&<limit>")]
pub async fn get_snapshots(
    state: &State<AppState>,
    _session: Session,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<Vec<SnapshotResp>> {
    let connection = state.db.connect()?;

    let page = page.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(25) as usize;

    let mut snapshots = models::Snapshot::query(
        &connection,
        models::Snapshot::query_select()
            .limit(limit)
            .offset(page * limit)
            .build()?,
    )
    .await?;

    let mut resp = Vec::new();

    for snapshot in snapshots.iter_mut() {
        snapshot.fetch_metadata(&connection).await?;

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
            created_at: snapshot.created_at,
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
                count = meta.as_string().parse().unwrap_or(0);
                continue;
            }
            metadata.insert(name.to_string(), meta.as_string());
        }

        SnapshotResp {
            id: snapshot.id.into(),
            created_at: snapshot.created_at,
            dependencies: count,
            security: SecuritySummary::default(),
            metadata,
        }
    }
}
