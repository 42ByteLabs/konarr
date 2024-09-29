use std::collections::HashMap;

use geekorm::prelude::*;
use konarr::{
    bom::{BomParser, Parsers},
    models,
};
use rocket::{data::ToByteUnit, serde::json::Json, State};

use super::{dependencies::DependencyResp, security::SecuritySummary, ApiResponse, ApiResult};
use crate::{guards::Session, AppState};

pub fn routes() -> Vec<rocket::Route> {
    routes![
        get_snapshot,
        get_snapshots,
        get_snapshot_dependencies,
        create_snapshot,
        upload_bom,
        patch_snapshot_metadata,
    ]
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct SnapshotResp {
    id: i32,
    created_at: String,
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

        snapshot.set_metadata(&connection, key, value).await?;
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
    let connection = state.db.connect()?;

    let mut snapshot = models::Snapshot::fetch_by_primary_key(&connection, id as i32).await?;

    // TODO: Implement file upload
    let data = data
        .open(2.megabytes())
        .into_bytes()
        .await
        .map_err(|_| konarr::KonarrError::ParseSBOM("Failed to read data".to_string()))?;
    let bom = Parsers::parse(&data)?;
    info!("Parsed SBOM: {:?}", bom);

    snapshot.add_bom(&connection, &bom).await?;

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
            if name == "bom.dependencies.count" {
                count = meta.as_string().parse().unwrap_or(0);
                continue;
            }
            metadata.insert(name.clone(), meta.as_string());
        }

        resp.push(SnapshotResp {
            id: snapshot.id.into(),
            created_at: snapshot.created_at.to_string(),
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
            if name == "bom.dependencies.count" {
                count = meta.as_string().parse().unwrap_or(0);
                continue;
            }
            metadata.insert(name.clone(), meta.as_string());
        }

        SnapshotResp {
            id: snapshot.id.into(),
            created_at: snapshot.created_at.to_string(),
            dependencies: count,
            security: SecuritySummary::default(),
            metadata,
        }
    }
}
