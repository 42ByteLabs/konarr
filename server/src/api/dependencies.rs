use geekorm::prelude::*;

use konarr::models;
use rocket::{serde::json::Json, State};

use super::{projects::ProjectResp, ApiResponse, ApiResult};
use crate::{guards::Session, AppState};

pub fn routes() -> Vec<rocket::Route> {
    routes![get_dependency, get_dependencies]
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub(crate) struct DependencyResp {
    id: i32,
    r#type: String,
    manager: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    purl: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    versions: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<Vec<ProjectResp>>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub(crate) struct DependencySummaryResp {
    pub count: i32,
}

/// Get single Dependency by ID
#[get("/<id>?<snapshot>")]
pub(crate) async fn get_dependency(
    state: &State<AppState>,
    _session: Session,
    id: i32,
    snapshot: Option<u32>,
) -> ApiResult<DependencyResp> {
    if let Some(snapshot_id) = snapshot {
        let mut dep = models::Dependencies::fetch_dependency_by_snapshot(
            &state.connection,
            snapshot_id as i32,
            id,
        )
        .await?;
        dep.fetch(&state.connection).await?;

        Ok(Json(dep.into()))
    } else {
        let mut dep = models::Component::fetch_by_primary_key(&state.connection, id).await?;
        dep.fetch(&state.connection).await?;

        let projects: Vec<ProjectResp> =
            models::Projects::find_project_by_component(&state.connection, dep.id.into())
                .await?
                .iter()
                .map(|p| p.clone().into())
                .collect();

        let versions: Vec<String> =
            models::ComponentVersion::fetch_by_component_id(&state.connection, dep.id)
                .await?
                .iter()
                .map(|v| v.clone().version)
                .collect();

        Ok(Json(DependencyResp {
            id: dep.id.into(),
            r#type: dep.component_type.to_string(),
            manager: dep.manager.to_string(),
            name: dep.name.to_string(),
            purl: Some(dep.purl()),
            projects: Some(projects),
            versions,
            ..Default::default()
        }))
    }
}

/// Get all Dependencies (components)
#[get("/?<search>&<top>&<deptype>&<page>&<limit>")]
pub async fn get_dependencies(
    state: &State<AppState>,
    _session: Session,
    search: Option<String>,
    top: Option<bool>,
    deptype: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<ApiResponse<Vec<DependencyResp>>> {
    let page = Page::from((page, limit));

    let deps = if let Some(search) = search {
        models::Component::find_by_name(&state.connection, search, &page).await?
    } else if let Some(dtyp) = deptype {
        models::Component::find_by_component_type(
            &state.connection,
            models::ComponentType::from(dtyp),
            &page,
        )
        .await?
    } else if top.unwrap_or(false) {
        models::Component::top(&state.connection, &page).await?
    } else {
        // Fetch all
        models::Component::query(
            &state.connection,
            models::Component::query_select().page(&page).build()?,
        )
        .await?
    };

    let total: u32 =
        models::Component::row_count(&state.connection, models::Component::query_count().build()?)
            .await? as u32;
    let pages = (total as f64 / page.limit() as f64).ceil() as u32;

    Ok(Json(ApiResponse::new(
        deps.iter().map(|dep| dep.clone().into()).collect(),
        total,
        pages,
    )))
}

impl From<models::Dependencies> for DependencyResp {
    fn from(dep: models::Dependencies) -> Self {
        DependencyResp {
            id: dep.component_id().into(),
            r#type: dep.component_type().to_string(),
            manager: dep.manager().to_string(),
            name: dep.name(),
            version: dep.version(),
            purl: Some(dep.purl()),
            ..Default::default()
        }
    }
}

impl From<models::Component> for DependencyResp {
    fn from(comp: models::Component) -> Self {
        DependencyResp {
            id: comp.id.into(),
            r#type: comp.component_type.to_string(),
            manager: comp.manager.to_string(),
            name: comp.name.to_string(),
            version: None,
            purl: Some(comp.purl()),
            ..Default::default()
        }
    }
}
