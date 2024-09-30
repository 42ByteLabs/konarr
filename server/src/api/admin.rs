use geekorm::prelude::*;
use std::collections::HashMap;

use konarr::models::{settings::ServerSettings, UserRole};
use log::info;
use rocket::{serde::json::Json, State};

use crate::{guards::Session, AppState};

use super::ApiResult;

pub fn routes() -> Vec<rocket::Route> {
    routes![settings, update_settings, get_users]
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AdminUserSummary {
    id: i32,
    username: String,
    state: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct AdminUserStats {
    total: i64,
    active: i64,
    inactive: i64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct AdminProjectStats {
    total: i64,
    inactive: i64,
    archived: i64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminResponse {
    pub settings: HashMap<String, String>,

    pub project_stats: AdminProjectStats,

    pub users: Vec<AdminUserSummary>,
    pub user_stats: AdminUserStats,
}

#[get("/")]
pub async fn settings(state: &State<AppState>, session: Session) -> ApiResult<AdminResponse> {
    let connection = state.db.connect()?;

    if session.user.role != UserRole::Admin {
        return Err(crate::error::KonarrServerError::Unauthorized);
    }

    let settings: Vec<ServerSettings> =
        ServerSettings::query(&connection, ServerSettings::query_all()).await?;

    let users =
        konarr::models::Users::query(&connection, konarr::models::Users::query_all()).await?;

    let user_stats = AdminUserStats {
        total: konarr::models::Users::total(&connection).await?,
        active: konarr::models::Users::count_active(&connection).await?,
        inactive: konarr::models::Users::count_inactive(&connection).await?,
    };
    let project_stats = AdminProjectStats {
        total: konarr::models::Projects::total(&connection).await?,
        inactive: konarr::models::Projects::count_inactive(&connection).await?,
        archived: konarr::models::Projects::count_archived(&connection).await?,
    };

    Ok(Json(AdminResponse {
        settings: settings
            .into_iter()
            .map(|setting| (setting.name.clone(), setting.value))
            .collect(),
        project_stats,
        user_stats,
        users: users
            .into_iter()
            .map(|user| AdminUserSummary {
                id: user.id.into(),
                username: user.username,
                state: user.state.to_string(),
                role: user.role.to_string(),
                created_at: user.created_at,
            })
            .collect(),
    }))
}

#[patch("/", data = "<settings>")]
pub async fn update_settings(
    state: &State<AppState>,
    session: Session,
    settings: Json<HashMap<String, String>>,
) -> ApiResult<AdminResponse> {
    let connection = state.db.connect()?;

    if session.user.role != UserRole::Admin {
        return Err(crate::error::KonarrServerError::Unauthorized);
    }
    info!("Updating settings: {:?}", settings);
    // TODO: "type" checking of the setting?

    for (name, value) in settings.iter() {
        let mut setting = ServerSettings::fetch_by_name(&connection, name).await?;

        setting.set(value);
        setting.update(&connection).await?;
    }

    let settings: Vec<ServerSettings> =
        ServerSettings::query(&connection, ServerSettings::query_all()).await?;
    let users =
        konarr::models::Users::query(&connection, konarr::models::Users::query_all()).await?;
    let user_stats = AdminUserStats {
        total: konarr::models::Users::total(&connection).await?,
        active: konarr::models::Users::count_active(&connection).await?,
        inactive: konarr::models::Users::count_inactive(&connection).await?,
    };
    let project_stats = AdminProjectStats {
        total: konarr::models::Projects::total(&connection).await?,
        inactive: konarr::models::Projects::count_inactive(&connection).await?,
        archived: konarr::models::Projects::count_archived(&connection).await?,
    };

    Ok(Json(AdminResponse {
        settings: settings
            .into_iter()
            .map(|setting| (setting.name.clone(), setting.value))
            .collect(),
        project_stats,
        user_stats,
        users: users
            .into_iter()
            .map(|user| AdminUserSummary {
                id: user.id.into(),
                username: user.username,
                state: user.state.to_string(),
                role: user.role.to_string(),
                created_at: user.created_at,
            })
            .collect(),
    }))
}

#[derive(serde::Serialize)]
pub struct UserResponse {
    pub username: String,
    pub role: String,
}

#[get("/users")]
pub(crate) async fn get_users(
    state: &State<AppState>,
    session: Session,
) -> ApiResult<Vec<UserResponse>> {
    let connection = state.db.connect()?;

    if session.user.role != UserRole::Admin {
        return Err(crate::error::KonarrServerError::Unauthorized);
    }

    let users =
        konarr::models::Users::query(&connection, konarr::models::Users::query_all()).await?;

    Ok(Json(
        users
            .into_iter()
            .map(|user| UserResponse {
                username: user.username,
                role: user.role.to_string(),
            })
            .collect(),
    ))
}
