use geekorm::prelude::*;
use konarr::models::Sessions;
use rocket::{State, serde::json::Json};

use super::ApiResult;
use crate::{AppState, error::KonarrServerError, guards::Session};

pub fn routes() -> Vec<rocket::Route> {
    routes![whoami, update_password, list_sessions, revoke_session]
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct UserResponse {
    /// Username of the user
    pub username: String,
    /// Avatar URL of the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Role of the user (Admin, User)
    pub role: String,
    /// User state (Active, Disabled, Reset)
    pub state: String,
    /// When the user account was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last login timestamp
    pub last_login: chrono::DateTime<chrono::Utc>,
}

#[get("/whoami")]
pub async fn whoami(_state: &State<AppState>, session: Session) -> ApiResult<UserResponse> {
    Ok(Json(UserResponse {
        username: session.user.username.clone(),
        avatar: None,
        role: session.user.role.to_string(),
        state: session.user.state.to_string(),
        created_at: session.user.created_at,
        last_login: session.user.last_login,
    }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct UpdatePasswordReq {
    pub current_password: String,
    pub new_password: String,
    pub new_password_confirm: String,
}

#[patch("/password", data = "<data>")]
pub async fn update_password(
    state: &State<AppState>,
    session: Session,
    data: Json<UpdatePasswordReq>,
) -> ApiResult<UserResponse> {
    let connection = state.connection().await;
    let mut user =
        konarr::models::Users::fetch_by_primary_key(&connection, session.user.id).await?;

    if data.new_password != data.new_password_confirm {
        return Err(KonarrServerError::KonarrError(
            konarr::KonarrError::AuthenticationError("New passwords do not match".to_string()),
        ));
    }

    // validate password length
    if data.new_password.len() < 12 {
        return Err(KonarrServerError::KonarrError(
            konarr::KonarrError::AuthenticationError(
                "Password must be at least 12 characters".to_string(),
            ),
        ));
    }

    // verify current password
    if !user.check_password(data.current_password.clone())? {
        return Err(KonarrServerError::KonarrError(
            konarr::KonarrError::AuthenticationError("Current password incorrect".to_string()),
        ));
    }

    // set the password (use the model helper used elsewhere)
    user.hash_password(data.new_password.clone())?;
    user.update(&connection).await?;

    Ok(Json(UserResponse {
        username: user.username,
        avatar: None,
        role: user.role.to_string(),
        state: user.state.to_string(),
        created_at: user.created_at,
        last_login: user.last_login,
    }))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct SessionSummary {
    pub id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub state: String,
}

#[get("/sessions")]
pub async fn list_sessions(
    state: &State<AppState>,
    session: Session,
) -> ApiResult<Vec<SessionSummary>> {
    let connection = state.connection().await;

    // The Sessions table can be queried for the sessions belonging to the user's session id.
    // There isn't a direct `Sessions::fetch_by_user` helper, so we'll return the current session
    let mut user =
        konarr::models::Users::fetch_by_primary_key(&connection, session.user.id).await?;
    let sess = user.fetch_sessions(&connection).await?;

    let out = vec![SessionSummary {
        id: sess.id.into(),
        created_at: sess.created_at,
        last_accessed: sess.last_accessed,
        state: sess.state.to_string(),
    }];

    Ok(Json(out))
}

#[delete("/sessions/<id>")]
pub async fn revoke_session(state: &State<AppState>, session: Session, id: i32) -> ApiResult<()> {
    let connection = state.connection().await;

    // Only allow revoking the current user's sessions (or admin via different endpoint)
    // Fetch the target session and ensure it belongs to the requesting user
    let mut target = Sessions::fetch_by_primary_key(&connection, id).await?;

    // Validate ownership: ensure the user's current session id matches the target or the user's sessions FK
    // We will fetch the user's session and compare token/ids
    let mut user =
        konarr::models::Users::fetch_by_primary_key(&connection, session.user.id).await?;
    let users_session = user.fetch_sessions(&connection).await?;

    if users_session.id != target.id {
        return Err(KonarrServerError::Unauthorized);
    }

    target.state = konarr::models::auth::sessions::SessionState::Inactive;
    target.update(&connection).await?;

    Ok(Json(()))
}
