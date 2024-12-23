use geekorm::prelude::*;
use konarr::models::{self, settings::ServerSettings, SessionState, SessionType, UserRole, Users};
use log::info;
use rocket::{http::CookieJar, serde::json::Json, State};
use rocket_governor::RocketGovernor;

use crate::{guards::Session, AppState};

use super::ApiResult;

pub fn routes() -> Vec<rocket::Route> {
    routes![login, logout, register]
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoginResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LogoutResponse {
    status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub password_confirm: String,
}

#[post("/login", data = "<payload>", format = "json")]
pub async fn login(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    payload: Json<LoginRequest>,
    _limiter: RocketGovernor<'_, crate::guards::limit::RateLimit>,
) -> ApiResult<LoginResponse> {
    let connection = std::sync::Arc::clone(&state.connection);

    let (user, session) = Users::login(
        &connection,
        payload.username.clone(),
        payload.password.clone(),
    )
    .await?;

    cookies.add_private(("x-konarr-token", session.token.clone()));

    log::info!("Successfull logged in: {:?}", user.id);
    if let Ok(mut sessions) = state.sessions.write() {
        log::debug!("Adding user session to in-memory cache - User({})", user.id);
        sessions.push(Session { user, session });
    }

    Ok(Json(LoginResponse::success()))
}

#[post("/logout")]
pub async fn logout(
    state: &State<AppState>,
    session: Session,
    cookies: &CookieJar<'_>,
) -> ApiResult<LogoutResponse> {
    let connection = std::sync::Arc::clone(&state.connection);

    let mut user = session.user.clone();
    user.logout(&connection).await?;

    cookies.remove_private("x-konarr-token");

    if let Ok(mut sessions) = state.sessions.write() {
        log::debug!(
            "Removing user session from in-memory cache - User({})",
            user.id
        );
        sessions.retain(|s| s.user.id != user.id);
    }

    Ok(Json(LogoutResponse {
        status: String::from("success"),
    }))
}

#[post("/register", data = "<payload>", format = "json")]
pub async fn register(
    state: &State<AppState>,
    session: Option<Session>,
    payload: Json<RegisterRequest>,
    _limiter: RocketGovernor<'_, crate::guards::limit::RateLimit>,
) -> ApiResult<LoginResponse> {
    if session.is_some() {
        return Ok(Json(LoginResponse::failed("Already logged in")));
    }
    let registration: String = ServerSettings::fetch_by_name(&state.connection, "registration")
        .await?
        .value;

    if registration == "enabled".to_string() {
        if payload.password != payload.password_confirm {
            return Ok(Json(LoginResponse::failed("Passwords do not match")));
        }

        let role = if !state.init {
            UserRole::Admin
        } else {
            UserRole::User
        };

        let mut session = models::Sessions::new(SessionType::User, SessionState::Active);
        session.save(&state.connection).await?;

        let mut user = Users::new(
            payload.username.clone(),
            payload.password.clone(),
            role,
            session.id,
        );
        user.save(&state.connection).await?;

        if !state.init {
            let mut deinit =
                ServerSettings::fetch_by_name(&state.connection, "initialized").await?;
            deinit.set_boolean("true");
            deinit.update(&state.connection).await?;
            info!("Server is now initialized");
        }

        let connection = std::sync::Arc::clone(&state.connection);
        tokio::spawn(async move {
            konarr::tasks::statistics(&connection)
                .await
                .map_err(|e| {
                    log::error!("Failed to run alert calculator: {:?}", e);
                })
                .ok();
        });

        Ok(Json(LoginResponse::success()))
    } else {
        Ok(Json(LoginResponse::failed("Registration is disabled")))
    }
}

impl LoginResponse {
    pub fn success() -> Self {
        Self {
            status: String::from("success"),
            reason: None,
        }
    }
    pub fn failed(reason: &str) -> Self {
        Self {
            status: String::from("failed"),
            reason: Some(reason.to_string()),
        }
    }
}
