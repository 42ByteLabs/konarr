//! # Guards
use konarr::models::{
    settings::{keys::Setting, ServerSettings},
    Sessions, UserRole, Users,
};
use rocket::{
    outcome::try_outcome,
    request::{FromRequest, Outcome, Request},
    State,
};

pub mod limit;

use crate::{error::KonarrServerError, AppState};

#[derive(Debug, Clone)]
pub struct Session {
    pub user: Users,
    #[allow(unused)]
    pub session: Sessions,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct AdminSession {
    pub user: Users,
    #[allow(unused)]
    pub session: Sessions,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let appstate: &State<AppState> = try_outcome!(req.guard::<&State<AppState>>().await);

        let connection = match appstate.db.connect() {
            Ok(connection) => connection,
            Err(_) => return Outcome::Error((rocket::http::Status::InternalServerError, ())),
        };

        // Agent
        if let Some(token) = req.headers().get_one("Authorization") {
            if agent_validation(appstate, &connection, &token).await {
                // This is a Agent User, no need to check the session
                // Return a dummy session
                return Outcome::Success(Session {
                    user: Users {
                        id: 0.into(),
                        username: "konarr-agent".to_string(),
                        role: UserRole::Agent,
                        ..Default::default()
                    },
                    session: Sessions::default(),
                });
            } else {
                return Outcome::Error((rocket::http::Status::Unauthorized, ()));
            }
        }

        // User Auth (Cookies)
        let token: String = if let Some(cookie) = req.cookies().get_private("x-konarr-token") {
            cookie.value().to_string()
        } else {
            return Outcome::Error((rocket::http::Status::Unauthorized, ()));
        };

        let session = match find_session(appstate, &connection, token.as_str()).await {
            Ok(session) => session,
            Err(e) => {
                log::warn!("Failed to get session: {}", e);
                return Outcome::Error((rocket::http::Status::Unauthorized, ()));
            }
        };

        log::info!("User performing action: {}", session.user.id);
        Outcome::Success(session)
    }
}

/// Find the session by token
///
/// - Checks the cached session
///   - Perform validity checks
/// - Checks the database
async fn find_session(
    appstate: &AppState,
    connection: &libsql::Connection,
    token: &str,
) -> Result<Session, KonarrServerError> {
    let config = &appstate.config.sessions();

    // Check the cached session (this is a quick check)
    if let Ok(sessions) = appstate.sessions.read() {
        if let Some(sess) = sessions.iter().find(|s| s.session.token == token) {
            log::debug!("Found session in cache - User({})", sess.user.id);
            if sess.user.validate_session(config) {
                return Ok(sess.clone());
            }
        }
    }

    // Check the database for the session (this is an expensive check)
    let session = match Sessions::fetch_by_token(connection, token.to_string()).await {
        Ok(session) => session,
        Err(_) => {
            log::error!("Provided session token is invalid");
            return Err(KonarrServerError::Unauthorized);
        }
    };

    let mut user = match Users::fetch_by_sessions(connection, session.id).await {
        Ok(user) => user.first().unwrap().clone(),
        Err(_) => return Err(KonarrServerError::InternalServerError),
    };
    user.sessions.data = session.clone();

    if !user.validate_session(&config) {
        log::error!("User session is invalid - User({})", user.id);
        return Err(KonarrServerError::Unauthorized);
    }

    match user.update_access(connection).await {
        Ok(_) => {
            log::debug!("Updated user access - User({})", user.id);
        }
        Err(_) => return Err(KonarrServerError::InternalServerError),
    };

    // Add the session to the cache
    if let Ok(mut sessions) = appstate.sessions.write() {
        log::debug!(
            "Adding session to cache - User({}); Sessions({})",
            user.id,
            session.id
        );
        sessions.push(Session {
            user: user.clone(),
            session: user.sessions.data.clone(),
        });
    }

    Ok(Session { user, session })
}

/// Validate the agent token
///
/// - Checks the cached token
/// - Checks the database for the token
async fn agent_validation(
    appstate: &AppState,
    connection: &libsql::Connection,
    token: &str,
) -> bool {
    // Check the cached agent token
    if let Ok(key) = &appstate.agent_token.read() {
        if token == **key {
            log::info!("Agent performing action");
            return true;
        }
        log::debug!("Cached Agent Key Mismatch, checking database");
    }
    // Check the database for the agent key (expensive check)
    match ServerSettings::fetch_by_name(connection, Setting::AgentKey).await {
        Ok(key) => {
            if token == key.value {
                log::info!("Agent performing action");
                return true;
            }
            if let Ok(mut atoken) = appstate.agent_token.write() {
                log::debug!("Updating cached agent key");
                *atoken = key.value.clone();
            }
        }
        _ => {}
    };
    log::error!("Invalid Agent Key");
    return false;
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminSession {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session: Session = try_outcome!(req.guard::<Session>().await);

        match session.user.role {
            UserRole::Admin => {
                log::info!("Admin performing action - Admin({})", session.user.id);
                Outcome::Success(AdminSession {
                    user: session.user,
                    session: session.session,
                })
            }
            UserRole::User | UserRole::Agent => {
                log::warn!(
                    "Non-Admin User tried performing action - User({})",
                    session.user.id
                );
                Outcome::Error((rocket::http::Status::Unauthorized, ()))
            }
        }
    }
}
