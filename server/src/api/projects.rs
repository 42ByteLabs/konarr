use geekorm::prelude::*;
use konarr::{
    models::{self, ProjectStatus, ProjectType},
    tasks::TaskTrait,
};
use rocket::{State, serde::json::Json};

use super::{ApiResponse, ApiResult, security::SecuritySummary};
use crate::{
    AppState,
    error::KonarrServerError,
    guards::{AdminSession, Pagination, Session},
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

    /// Total Snapshots count
    snapshots: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<super::snapshots::SnapshotResp>,

    /// Total Alerts count
    alerts: u32,
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

    if project.status == ProjectStatus::Archived {
        log::info!("Tried accessing an archived project: {}", project.id);
        Err(KonarrServerError::ProjectNotFoundError(id))
    } else {
        // Fetch Children
        project.fetch_children(&connection).await?;
        // Fetch the latest snapshot for the current project
        project.fetch_latest_snapshot(&connection).await?;

        log::info!(
            "{:?} (children: {}, snapshots: {}, latest: {})",
            project.id,
            project.children.len(),
            project.snapshot_count.unwrap_or_default(),
            project.latest_snapshot().map(|s| s.id.into()).unwrap_or(0)
        );

        Ok(Json(project.into()))
    }
}

#[get("/?<search>&<select>&<top>&<parents>")]
pub(crate) async fn get_projects(
    state: &State<AppState>,
    _session: Session,
    pagination: Pagination,
    search: Option<String>,
    top: Option<bool>,
    select: Option<String>,
    parents: Option<bool>,
) -> ApiResult<ApiResponse<Vec<ProjectResp>>> {
    // Build Query
    let mut query = models::Projects::query_select()
        .where_eq("status", ProjectStatus::Active)
        .order_by("created_at", QueryOrder::Desc);

    // Search by name
    if let Some(search) = search {
        log::info!("Searching for projects with name: '{}'", search);
        query = query.and().where_like("name", format!("%{}%", search));
    }
    // Filter by parents or top level projects
    if parents.unwrap_or(false) {
        log::info!("Get the parent projects");
        query = query.and().where_gt("parent", 0);
    } else if top.unwrap_or(false) {
        log::info!("Fetching the top level projects");
        query = query.and().where_eq("parent", 0);
    }
    // Filter by project type
    if let Some(prjtype) = select {
        if prjtype.as_str() == "all" {
            log::info!("Fetching all projects");
            // No additional filtering
        } else {
            log::info!("Fetching by type: {}", prjtype);
            query = query
                .and()
                .where_eq("project_type", ProjectType::from(prjtype));
        }
    }

    // TODO: The total should be based on the filtered query
    let total = models::ProjectStatus::count_active(&state.connection().await).await?;
    let page = pagination.page_with_total(total as u32);

    let mut projects =
        models::Projects::query(&state.connection().await, query.page(&page).build()?).await?;
    for project in projects.iter_mut() {
        // Fetch Children
        project.fetch_children(&state.connection().await).await?;
        // Fetch the latest snapshot for the current project
        project
            .fetch_latest_snapshot(&state.connection().await)
            .await?;
    }
    let count = projects.len();
    log::info!("Fetched {} projects", projects.len());

    Ok(Json(ApiResponse::new(
        projects.into_iter().map(|p| p.into()).collect(),
        total as u32,
        count as u32,
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

    konarr::tasks::StatisticsTask::spawn(&state.database).await?;
    konarr::tasks::ProjectsTask::spawn(&state.database).await?;

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
    pub(crate) parent: Option<i32>,
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
        log::info!("Updating Project (title) :: {}", title);
        project.title = Some(title.clone());
    }
    if let Some(typ) = &project_req.project_type {
        log::info!("Updating Project (type) :: {}", typ);
        project.project_type = ProjectType::from(typ.clone());
    }
    if let Some(desc) = &project_req.description {
        log::info!("Update Project (description) :: {}", desc);
        if desc.is_empty() {
            project.description = None;
        } else {
            project.description = Some(desc.clone());
        }
    }
    if let Some(parent) = &project_req.parent {
        if parent != project.id.value() {
            log::info!("Updating Project (parent) :: {}", parent);
            project.parent = *parent;
        }
        // TODO: Update the name of the project?
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
    let connection = state.connection().await;

    let mut project = models::Projects::fetch_by_primary_key(&connection, id).await?;
    // Fetch Children and Latest Snapshot
    project.fetch_children(&connection).await?;
    project.fetch_snapshots(&connection).await?;
    log::info!("{:?} (snapshots: {})", project.id, project.snapshots.len());

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
    log::info!(
        "Archiving Project :: {} by {}",
        project.name,
        session.user.username
    );
    project.archive(&connection).await?;

    konarr::tasks::StatisticsTask::spawn(&state.database).await?;
    konarr::tasks::ProjectsTask::spawn(&state.database).await?;
    konarr::tasks::AlertCalculatorTask::spawn(&state.database).await?;

    Ok(Json(project.into()))
}

/// Model -> Response
impl From<models::Projects> for ProjectResp {
    fn from(project: models::Projects) -> Self {
        // Get the latest snapshot (last)
        let snapshot: Option<models::Snapshot> = project.latest_snapshot();
        let security: Option<SecuritySummary> = snapshot.as_ref().map(|snap| snap.into());

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
            snapshots: project.snapshot_count.unwrap_or_default() as u32,
            alerts: security.as_ref().map_or(0, |s| s.total),
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
