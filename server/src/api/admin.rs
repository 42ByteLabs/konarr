use geekorm::prelude::*;
use konarr::models::{
    auth::users::UserState,
    settings::{keys::Setting, ServerSettings, SettingType},
};
use log::{info, warn};
use rocket::{serde::json::Json, State};
use std::collections::HashMap;

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
    total: u64,
    active: u64,
    inactive: u64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AdminProjectStats {
    total: u64,
    inactive: u64,
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
    log::info!("Fetching server settings");
    let settings = ServerSettings::fetch_settings(&state.connection).await?;
    log::debug!("Fetched {} settings", settings.len());
    let stats = ServerSettings::fetch_statistics(&state.connection).await?;
    log::debug!("Fetched {} stats", stats.len());

    // TODO: This will get all the users, we should limit this?
    let users =
        konarr::models::Users::query(&state.connection, konarr::models::Users::query_all()).await?;

    let user_stats = AdminUserStats::from(&stats);
    let project_stats = AdminProjectStats::from(&stats);

    Ok(Json(AdminResponse {
        settings: settings
            .into_iter()
            .map(|setting| (setting.name.to_string(), setting.value))
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
    info!("Updating settings: {:?}", settings);

    for (name, value) in settings.iter() {
        let mut setting = ServerSettings::fetch_by_name(&state.connection, name).await?;

        match setting.setting_type {
            SettingType::Toggle | SettingType::Regenerate | SettingType::SetString => {
                setting.set(value);
                setting.update(&state.connection).await?;
            }
            _ => {
                warn!("Read-only Server Setting is being updated: {}", name);
                return Err(KonarrServerError::UnauthorizedReadonly(name.to_string()));
            }
        }
    }

    // TODO: Return updated settings
    let stats = konarr::models::ServerSettings::fetch_statistics(&state.connection).await?;
    let settings = ServerSettings::fetch_settings(&state.connection).await?;

    let users =
        konarr::models::Users::query(&state.connection, konarr::models::Users::query_all()).await?;

    let user_stats = AdminUserStats::from(&stats);
    let project_stats = AdminProjectStats::from(&stats);

    Ok(Json(AdminResponse {
        settings: settings
            .into_iter()
            .map(|setting| (setting.name.to_string(), setting.value))
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
    let users =
        konarr::models::Users::query(&state.connection, konarr::models::Users::query_all()).await?;

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
    let mut user =
        konarr::models::Users::fetch_by_primary_key(&state.connection, id as i32).await?;
    log::info!("Updating user :: {}", user.username);

    // The default user cannot be changed
    if user.id == 1.into() {
        return Err(KonarrServerError::Unauthorized);
    }

    if let Some(ustate) = &data.state {
        let og = user.state.clone();
        user.state = match ustate.to_lowercase().as_str() {
            "active" => UserState::Active,
            "disabled" => UserState::Disabled,
            _ => {
                return Err(KonarrServerError::InternalServerError);
            }
        };
        log::info!("Updating users state - `{:?}` -> `{:?}`", og, user.state);

        if user.state == konarr::models::auth::users::UserState::Disabled {
            // Logout the user
            user.logout(&state.connection).await?;
        }
    }
    if let Some(role) = &data.role {
        user.role = konarr::models::UserRole::from(role.as_str());
        log::info!("Updating user role to :: {:?}", user.role);
    }

    user.update(&state.connection).await?;

    Ok(Json(AdminUserSummary {
        id: user.id.into(),
        username: user.username,
        role: user.role.to_string(),
        state: user.state.to_string(),
        created_at: user.created_at,
    }))
}

impl From<&Vec<ServerSettings>> for AdminUserStats {
    fn from(value: &Vec<ServerSettings>) -> Self {
        let mut stats = AdminUserStats::default();
        for setting in value {
            match setting.name {
                Setting::StatsUsersTotal => stats.total = setting.value.parse().unwrap_or(0),
                Setting::StatsUsersActive => stats.active = setting.value.parse().unwrap_or(0),
                Setting::StatsUsersInactive => stats.inactive = setting.value.parse().unwrap_or(0),
                _ => {}
            }
        }
        stats
    }
}

impl From<&Vec<ServerSettings>> for AdminProjectStats {
    fn from(value: &Vec<ServerSettings>) -> Self {
        let mut stats = AdminProjectStats::default();
        for setting in value {
            match setting.name {
                Setting::StatsProjectsTotal => stats.total = setting.value.parse().unwrap_or(0),
                Setting::StatsProjectsInactive => {
                    stats.inactive = setting.value.parse().unwrap_or(0)
                }
                Setting::StatsProjectsArchived => {
                    stats.archived = setting.value.parse().unwrap_or(0)
                }
                _ => {}
            }
        }
        stats
    }
}
