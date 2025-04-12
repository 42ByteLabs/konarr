use geekorm::prelude::*;
use konarr::{
    models::{self, ProjectType},
    tasks::TaskTrait,
};
use log::info;
use rocket::{State, serde::json::Json};

use super::{ApiResponse, ApiResult, security::SecuritySummary};
use crate::{
    AppState,
    error::KonarrServerError,
    guards::{AdminSession, Session},
};

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
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
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

    created_at: chrono::DateTime<chrono::Utc>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<ProjectResp>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
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
    let connection = state.connection().await;
    let mut project = models::Projects::fetch_by_primary_key(&connection, id).await?;

    if project.status == models::ProjectStatus::Archived {
        info!("Tried accessing an archived project: {}", project.id);
        Err(KonarrServerError::ProjectNotFoundError(id))
    } else {
        // Fetch Children
        project.fetch_children(&connection).await?;
        // Fetch the latest snapshot for the current project
        project.fetch_latest_snapshot(&connection).await?;

        info!("{:?} (snapshots: {})", project.id, project.snapshots.len());

        Ok(Json(project.into()))
    }
}

#[get("/?<page>&<limit>&<search>&<type>&<top>&<parents>")]
pub(crate) async fn get_projects(
    state: &State<AppState>,
    _session: Session,
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    top: Option<bool>,
    r#type: Option<String>,
    parents: Option<bool>,
) -> ApiResult<ApiResponse<Vec<ProjectResp>>> {
    let connection = state.connection().await;
    let total = models::Projects::count_active(&connection).await?;
    let mut page = Page::from((page, limit));
    page.set_total(total as u32);

    let projects = if let Some(search) = search {
        info!("Searching for projects with name: '{}'", search);
        models::Projects::search_title(&connection, search).await?
    } else if parents.unwrap_or(false) {
        info!("Get the parent projects");
        models::Projects::find_parents(&connection).await?
    } else if top.unwrap_or(false) {
        info!("Fetching the top level projects");
        models::Projects::fetch_top_level(&connection, &page).await?
        // state.projects.read_page(&page)?
    } else if let Some(prjtype) = r#type {
        if prjtype.as_str() == "all" {
            info!("Fetching all projects");
            models::Projects::page(&connection, &page).await?
        } else {
            info!("Fetching by type: {}", prjtype);
            models::Projects::fetch_project_type(&connection, prjtype, &page).await?
        }
    } else {
        models::Projects::page(&connection, &page).await?
    };

    log::debug!("Database - Get Projects :: {}", connection.count());
    konarr::tasks::StatisticsTask::spawn(&state.database).await?;

    Ok(Json(ApiResponse::new(
        projects.into_iter().map(|p| p.into()).collect(),
        total as u32,
        page.pages(),
    )))
}

#[post("/", data = "<project_req>", format = "json")]
pub async fn create_project(
    state: &State<AppState>,
    _session: Session,
    project_req: Json<ProjectReq>,
) -> ApiResult<ProjectResp> {
    log::info!("Creating Project: `{}`", project_req.name);
    let connection = state.connection().await;

    let mut project: models::Projects = project_req.into_inner().into();
    project.fetch_or_create(&connection).await?;

    // konarr::tasks::StatisticsTask::spawn(&state.database).await?;
    Ok(Json(project.into()))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
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
    let connection = state.connection().await;

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
        info!("Updating Project (title) :: {}", title);
        project.title = Some(title.clone());
    }
    if let Some(typ) = &project_req.project_type {
        info!("Updating Project (type) :: {}", typ);
        project.project_type = ProjectType::from(typ.clone());
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
        // TODO: Update the name of the project?
    }

    project.update(&connection).await?;

    // konarr::tasks::StatisticsTask::spawn(&state.database).await?;

    Ok(Json(project.into()))
}

#[patch("/<id>/metadata")]
#[allow(unused)]
pub(crate) async fn update_project_metadata(
    state: &State<AppState>,
    _session: Session,
    id: i32,
) -> ApiResult<ProjectResp> {
    let connection = state.connection().await;

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
    session: AdminSession,
    id: i32,
) -> ApiResult<ProjectResp> {
    let connection = state.connection().await;

    let mut project = match models::Projects::fetch_by_primary_key(&connection, id).await {
        Ok(project) => project,
        Err(_) => return Err(KonarrServerError::ProjectNotFoundError(id)),
    };
    info!(
        "Archiving Project :: {} by {}",
        project.name, session.user.username
    );
    project.archive(&connection).await?;

    konarr::tasks::StatisticsTask::spawn(&state.database).await?;

    Ok(Json(project.into()))
}

/// Model -> Response
impl From<models::Projects> for ProjectResp {
    fn from(project: models::Projects) -> Self {
        // Get the latest snapshot (last)
        let snapshot: Option<models::Snapshot> = project.snapshots.last().cloned();
        let security: Option<SecuritySummary> = if let Some(snap) = &snapshot {
            Some(snap.into())
        } else {
            None
        };
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
            created_at: project.created_at,
            snapshot: snapshot.map(|snap| snap.into()),
            snapshots: project.snapshots.len() as u32,
            security,
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
        let title = project
            .name
            .split('/')
            .last()
            .unwrap_or(project.name.as_str())
            .to_string();

        models::Projects {
            name: project.name.clone(),
            title: Some(title),
            project_type: ProjectType::from(project.r#type),
            description: project.description,
            created_at: chrono::Utc::now(),
            parent: project.parent.unwrap_or(0),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_req_to_project() {
        let req = ProjectReq {
            name: "server/test".to_string(),
            r#type: "server".to_string(),
            description: Some("test".to_string()),
            parent: Some(1),
        };

        let project: models::Projects = req.into();
        assert_eq!(project.name.as_str(), "server/test");
        assert_eq!(project.title, Some("test".to_string()));
        assert_eq!(project.project_type, models::ProjectType::Server);
    }
}
