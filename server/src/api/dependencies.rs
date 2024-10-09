use geekorm::prelude::*;

use konarr::models;
use rocket::{serde::json::Json, State};

use super::{projects::ProjectResp, ApiResponse, ApiResult};
use crate::{guards::Session, AppState};

pub fn routes() -> Vec<rocket::Route> {
    routes![get_dependency, get_dependencies]
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
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

    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<Vec<ProjectResp>>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
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
    let connection = state.db.connect()?;

    if let Some(snapshot_id) = snapshot {
        let mut dep =
            models::Dependencies::fetch_dependency_by_snapshot(&connection, snapshot_id as i32, id)
                .await?;
        dep.fetch(&connection).await?;

        Ok(Json(dep.into()))
    } else {
        let mut dep = models::Component::fetch_by_primary_key(&connection, id).await?;
        dep.fetch(&connection).await?;

        let projects: Vec<ProjectResp> =
            models::Projects::find_project_by_component(&connection, dep.id.into())
                .await?
                .iter()
                .map(|p| p.clone().into())
                .collect();

        Ok(Json(DependencyResp {
            id: dep.id.into(),
            r#type: dep.component_type.to_string(),
            manager: dep.manager.to_string(),
            name: dep.name.to_string(),
            purl: Some(dep.purl()),
            projects: Some(projects),
            ..Default::default()
        }))
    }
}

/// Get all Dependencies (components)
#[get("/?<search>&<page>&<limit>")]
pub async fn get_dependencies(
    state: &State<AppState>,
    _session: Session,
    search: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
) -> ApiResult<ApiResponse<Vec<DependencyResp>>> {
    let connection = state.db.connect()?;

    let page = page.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(25) as usize;

    let deps = if let Some(search) = search {
        models::Component::find_by_name(&connection, search, page, limit).await?
    } else {
        // Fetch all
        models::Component::query(
            &connection,
            models::Component::query_select()
                .limit(limit)
                .offset(page * limit)
                .build()?,
        )
        .await?
    };

    let total: u32 =
        models::Component::row_count(&connection, models::Component::query_count().build()?).await?
            as u32;
    let pages = (total as f64 / limit as f64).ceil() as u32;

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
