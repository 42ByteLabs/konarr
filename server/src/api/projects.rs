use geekorm::prelude::*;
use konarr::models::{self, ProjectType};
use rocket::{serde::json::Json, State};

use super::{ApiResponse, ApiResult};
use crate::{error::KonarrServerError, guards::Session, AppState};

pub fn routes() -> Vec<rocket::Route> {
    routes![
        // GET /projects/<id>
        get_project,
        // GET /projects
        get_projects,
        // POST /projects
        create_project,
        patch_project,
        delete_project,
    ]
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectResp {
    id: i32,
    name: String,
    title: String,
    #[serde(rename = "type")]
    project_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<super::snapshots::SnapshotResp>,
    snapshots: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    security: Option<super::security::SecuritySummary>,

    created_at: String,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<ProjectResp>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectReq {
    name: String,
    #[serde(rename = "type")]
    r#type: String,
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<i32>,
}

#[get("/<id>")]
pub(crate) async fn get_project(
    state: &State<AppState>,
    _session: Session,
    id: i32,
) -> ApiResult<ProjectResp> {
    let connection = state.db.connect()?;

    let mut project = models::Projects::fetch_by_primary_key(&connection, id).await?;

    if project.status == models::ProjectStatus::Archived {
        info!("Tried accessing an archived project: {}", project.id);
        Err(KonarrServerError::ProjectNotFoundError(id))
    } else {
        // Fetch Children and Latest Snapshot
        project.fetch_children(&connection).await?;
        project.fetch_snapshots(&connection).await?;

        info!("{:?} (snapshots: {})", project.id, project.snapshots.len());

        Ok(Json(project.into()))
    }
}

#[get("/?<page>&<limit>&<search>&<top>&<parents>")]
pub(crate) async fn get_projects(
    state: &State<AppState>,
    _session: Session,
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    top: Option<bool>,
    parents: Option<bool>,
) -> ApiResult<ApiResponse<Vec<ProjectResp>>> {
    let connection = state.db.connect()?;

    let limit = limit.unwrap_or(10) as usize;
    let offset = page.unwrap_or(0) as usize * limit as usize;

    let total = models::Projects::count_active(&connection).await?;
    let pages = (total as f32 / limit as f32).ceil() as u32;

    let projects = if let Some(search) = search {
        info!("Searching for projects with name: '{}'", search);
        models::Projects::search(&connection, search).await?
    } else if parents.unwrap_or(false) {
        models::Projects::find_parents(&connection).await?
    } else if top.unwrap_or(false) {
        info!("Fetching the top level projects");
        models::Projects::fetch_top_level(&connection, limit, offset).await?
    } else {
        models::Projects::query(
            &connection,
            models::Projects::query_select()
                .order_by("created_at", geekorm::QueryOrder::Desc)
                .limit(limit)
                .offset(offset)
                .build()?,
        )
        .await?
    };

    Ok(Json(ApiResponse::new(
        projects.into_iter().map(|p| p.into()).collect(),
        total as u32,
        pages,
    )))
}

#[post("/", data = "<project_req>", format = "json")]
pub async fn create_project(
    state: &State<AppState>,
    _session: Session,
    project_req: Json<ProjectReq>,
) -> ApiResult<ProjectResp> {
    let connection = state.db.connect()?;

    let mut project: models::Projects = project_req.into_inner().into();

    match project.fetch_or_create(&connection).await {
        Ok(_) => Ok(Json(project.into())),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdateRequest {
    pub(crate) id: Option<u32>,
    pub(crate) title: Option<String>,
    #[serde(rename = "type")]
    pub(crate) project_type: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) parent: Option<u32>,
}

#[patch("/<id>", data = "<project_req>", format = "json")]
pub async fn patch_project(
    state: &State<AppState>,
    _session: Session,
    project_req: Json<ProjectUpdateRequest>,
    id: Option<u32>,
) -> ApiResult<ProjectResp> {
    let connection = state.db.connect()?;

    let project_id = if let Some(id) = id {
        id
    } else if let Some(id) = project_req.id {
        id
    } else {
        0
    };

    let mut project =
        models::Projects::fetch_by_primary_key(&connection, project_id as i32).await?;

    if let Some(title) = &project_req.title {
        info!("Updating Project (name) :: {}", title);
        project.title = Some(title.clone());
    }
    if let Some(typ) = &project_req.project_type {
        info!("Updating Project (type) :: {}", typ);
        project.project_type = ProjectType::from(typ);
    }
    if let Some(desc) = &project_req.description {
        info!("Update Project (description) :: {}", desc);
        if desc.is_empty() {
            project.description = None;
        } else {
            project.description = Some(desc.clone());
        }
    }
    if let Some(parent) = &project_req.parent {
        project.parent = *parent as i32;
    }

    project.update(&connection).await?;

    Ok(Json(project.into()))
}

#[patch("/<id>/metadata")]
#[allow(unused)]
pub(crate) async fn update_project_metadata(
    state: &State<AppState>,
    _session: Session,
    id: i32,
) -> ApiResult<ProjectResp> {
    let connection = state.db.connect()?;

    let mut project = models::Projects::fetch_by_primary_key(&connection, id).await?;
    // Fetch Children and Latest Snapshot
    project.fetch_children(&connection).await?;
    project.fetch_snapshots(&connection).await?;
    info!("{:?} (snapshots: {})", project.id, project.snapshots.len());

    Ok(Json(project.into()))
}

#[delete("/<id>")]
pub async fn delete_project(
    state: &State<AppState>,
    session: Session,
    id: i32,
) -> ApiResult<ProjectResp> {
    let connection = state.db.connect()?;

    if session.user.role == models::UserRole::Admin {
        let mut project = match models::Projects::fetch_by_primary_key(&connection, id).await {
            Ok(project) => project,
            Err(_) => return Err(KonarrServerError::ProjectNotFoundError(id)),
        };
        info!("Archiving Project :: {}", project.name);
        project.archive(&connection).await?;

        Ok(Json(project.into()))
    } else {
        return Err(KonarrServerError::Unauthorized);
    }
}

/// Model -> Response
impl From<models::Projects> for ProjectResp {
    fn from(project: models::Projects) -> Self {
        // Get the latest snapshot (last)
        let snapshot: Option<models::Snapshot> = project.snapshots.last().cloned();
        let parent: Option<i32> = if project.parent > 0 {
            Some(project.parent)
        } else {
            None
        };

        let status: Option<bool> = match &snapshot {
            Some(snap) => snap
                .find_metadata("status")
                .map(|status| status.value == "online".as_bytes()),
            None => None,
        };

        ProjectResp {
            id: project.id.into(),
            name: project.name.clone(),
            title: project.title.unwrap_or(project.name),
            status,
            project_type: project.project_type.to_string(),
            description: project.description.clone(),
            created_at: project.created_at.to_string(),
            snapshot: snapshot.map(|snap| snap.into()),
            snapshots: project.snapshots.len() as u32,
            security: Some(super::security::SecuritySummary::default()),
            parent,
            children: project
                .children
                .iter()
                .map(|proj| proj.clone().into())
                .collect(),
            ..Default::default()
        }
    }
}

/// Request -> Model
impl From<ProjectReq> for models::Projects {
    fn from(project: ProjectReq) -> Self {
        models::Projects {
            name: project.name.clone(),
            title: Some(project.name),
            project_type: ProjectType::from(project.r#type),
            description: project.description,
            created_at: chrono::Utc::now(),
            parent: project.parent.unwrap_or(0),
            ..Default::default()
        }
    }
}
