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
    let connection = state.db.connect()?;

    let mut user: Users = Users::fetch_by_username(&connection, payload.username.clone()).await?;

    if !user.check_password(&payload.password)? {
        Ok(Json(LoginResponse::failed("Invalid credentials")))
    } else {
        info!("Logging in user: {:?}", user.id);
        let mut session = user.fetch_sessions(&connection).await?;
        session.state = SessionState::Active;
        session.regenerate_token();
        session.last_accessed = chrono::Utc::now();
        session.update(&connection).await?;

        cookies.add_private(("x-konarr-token", session.token));

        Ok(Json(LoginResponse::success()))
    }
}

#[post("/logout")]
pub async fn logout(
    state: &State<AppState>,
    session: Session,
    cookies: &CookieJar<'_>,
) -> ApiResult<LogoutResponse> {
    let connection = state.db.connect()?;

    let mut user = session.user.clone();
    user.logout(&connection).await?;

    cookies.remove_private("x-konarr-token");

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
    let connection = state.db.connect()?;
    let registration: String = ServerSettings::fetch_by_name(&connection, "registration")
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
        session.save(&connection).await?;

        let mut user = Users::new(
            payload.username.clone(),
            payload.password.clone(),
            role,
            session.id,
        );
        user.save(&connection).await?;

        if !state.init {
            let mut deinit = ServerSettings::fetch_by_name(&connection, "initialized").await?;
            deinit.set_boolean("true");
            deinit.update(&connection).await?;
            info!("Server is now initialized");
        }

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
