use geekorm::prelude::*;
use std::collections::HashMap;

use konarr::models::settings::ServerSettings;
use log::info;
use rocket::{serde::json::Json, State};

use crate::{error::KonarrServerError, guards::AdminSession, AppState};

use super::ApiResult;

pub fn routes() -> Vec<rocket::Route> {
    routes![
        settings,
        update_settings,
        // Users
        get_users,
        update_users,
    ]
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AdminUserSummary {
    id: i32,
    username: String,
    state: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AdminUserStats {
    total: i64,
    active: i64,
    inactive: i64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AdminProjectStats {
    total: i64,
    inactive: i64,
    archived: i64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AdminResponse {
    pub settings: HashMap<String, String>,

    pub project_stats: AdminProjectStats,

    pub users: Vec<AdminUserSummary>,
    pub user_stats: AdminUserStats,
}

#[get("/")]
pub async fn settings(state: &State<AppState>, _session: AdminSession) -> ApiResult<AdminResponse> {
    let connection = state.db.connect()?;

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
    _session: AdminSession,
    settings: Json<HashMap<String, String>>,
) -> ApiResult<AdminResponse> {
    let connection = state.db.connect()?;

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

#[get("/users")]
pub(crate) async fn get_users(
    state: &State<AppState>,
    _session: AdminSession,
) -> ApiResult<Vec<AdminUserSummary>> {
    let connection = state.db.connect()?;

    let users =
        konarr::models::Users::query(&connection, konarr::models::Users::query_all()).await?;

    Ok(Json(
        users
            .into_iter()
            .map(|user| AdminUserSummary {
                id: user.id.into(),
                username: user.username,
                role: user.role.to_string(),
                state: user.state.to_string(),
                created_at: user.created_at,
            })
            .collect(),
    ))
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPatchReq {
    // Update the role of a user
    role: Option<String>,
    // Status
    state: Option<String>,
}

#[patch("/users/<id>", data = "<data>")]
pub(crate) async fn update_users(
    state: &State<AppState>,
    _session: AdminSession,
    id: u32,
    data: Json<UserPatchReq>,
) -> ApiResult<AdminUserSummary> {
    let connection = state.db.connect()?;

    let mut user = konarr::models::Users::fetch_by_primary_key(&connection, id as i32).await?;
    log::info!("Updating user :: {}", user.username);

    // The default user cannot be changed
    if user.id == 1.into() {
        return Err(KonarrServerError::Unauthorized);
    }

    if let Some(state) = &data.state {
        let og = user.state.clone();
        user.state = konarr::models::auth::users::UserState::from(state.as_str());
        log::info!("Updating users state to `{:?}` from `{:?}`", user.state, og);

        if user.state == konarr::models::auth::users::UserState::Disabled {
            // Logout the user
            user.logout(&connection).await?;
        }
    }
    if let Some(role) = &data.role {
        user.role = konarr::models::UserRole::from(role.as_str());
        log::info!("Updating user role to :: {:?}", user.role);
    }

    user.update(&connection).await?;

    Ok(Json(AdminUserSummary {
        id: user.id.into(),
        username: user.username,
        role: user.role.to_string(),
        state: user.state.to_string(),
        created_at: user.created_at,
    }))
}
