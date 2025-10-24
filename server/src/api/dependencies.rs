use std::str::FromStr;

use geekorm::prelude::*;

use konarr::models;
use rocket::{State, serde::json::Json};

use super::{ApiResponse, ApiResult, projects::ProjectResp};
use crate::{
    AppState,
    guards::{Pagination, Session},
};

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

/// Get single Dependency by ID
#[get("/<id>?<snapshot>")]
pub(crate) async fn get_dependency(
    state: &State<AppState>,
    _session: Session,
    id: i32,
    snapshot: Option<u32>,
) -> ApiResult<DependencyResp> {
    let connection = state.connection().await;

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

        let versions: Vec<String> =
            models::ComponentVersion::fetch_by_component_id(&connection, dep.id)
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
#[get("/?<search>&<top>&<select>")]
pub async fn get_dependencies(
    state: &State<AppState>,
    _session: Session,
    search: Option<String>,
    top: Option<bool>,
    select: Option<String>,
    pagination: Pagination,
) -> ApiResult<ApiResponse<Vec<DependencyResp>>> {
    let total: u32 = models::Component::row_count(
        &state.connection().await,
        models::Component::query_count().build()?,
    )
    .await? as u32;

    let page = pagination.page_with_total(total);

    let mut query = models::Component::query_select().order_by("name", QueryOrder::Asc);

    if let Some(search) = search {
        log::info!("Searching for components with name: '{}'", search);
        query = query.where_like("name", format!("%{}%", search));
    }
    if let Some(dtyp) = select {
        log::info!("Filtering components by type: '{}'", dtyp);
        query = query.where_eq("component_type", models::ComponentType::from_str(&dtyp)?);
    } else if top.unwrap_or(false) {
        // Fetch top level
        log::info!("Fetching the top level components");
        query = query
            .where_ne("component_type", models::ComponentType::Library)
            .and()
            .where_ne("component_type", models::ComponentType::Unknown)
            .and()
            .where_ne("component_type", models::ComponentType::Framework)
    }

    let deps = models::Component::query(&state.connection().await, query.build()?).await?;

    Ok(Json(ApiResponse::new(
        deps.iter().map(|dep| dep.clone().into()).collect(),
        total,
        deps.len() as u32,
        page.pages(),
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
