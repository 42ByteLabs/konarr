use konarr::models::{settings::ServerSettings, Sessions, UserRole, Users};
use rocket::{
    outcome::try_outcome,
    request::{FromRequest, Outcome, Request},
    State,
};

pub mod limit;

use crate::AppState;

#[derive(Debug)]
pub struct Session {
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
            match ServerSettings::fetch_by_name(&connection, "agent.key").await {
                Ok(key) => {
                    // Match Agent Key
                    if token != key.value {
                        log::error!("Invalid Agent Key");
                        return Outcome::Error((rocket::http::Status::Unauthorized, ()));
                    }

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
                }
                _ => {}
            };
        }

        // User Auth (Cookies)
        let token: String = if let Some(cookie) = req.cookies().get_private("x-konarr-token") {
            cookie.value().to_string()
        } else {
            return Outcome::Error((rocket::http::Status::Unauthorized, ()));
        };

        let session: Sessions = match Sessions::fetch_by_token(&connection, token.to_string()).await
        {
            Ok(session) => session,
            Err(_) => return Outcome::Error((rocket::http::Status::Unauthorized, ())),
        };

        let mut user: Users = match Users::fetch_by_sessions(&connection, session.id).await {
            Ok(user) => user.first().unwrap().clone(),
            Err(_) => return Outcome::Error((rocket::http::Status::Unauthorized, ())),
        };
        user.sessions.data = session.clone();

        let config = &appstate.config.sessions();

        if !user.validate_session(&connection, &config).await {
            return Outcome::Error((rocket::http::Status::Unauthorized, ()));
        }

        match user.update_access(&connection).await {
            Ok(_) => {}
            Err(_) => return Outcome::Error((rocket::http::Status::InternalServerError, ())),
        };

        Outcome::Success(Session { user, session })
    }
}
