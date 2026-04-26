//! # Users Models
use geekorm::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    KonarrError,
    utils::config::{SessionsConfig, SessionsRoleConfig},
};

use super::sessions::{SessionState, SessionType, Sessions};

/// Users Model / Table
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Users {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKeyInteger,

    /// User State
    #[geekorm(new = "UserState::default()")]
    pub state: UserState,

    /// Username
    #[geekorm(unique, not_null)]
    pub username: String,

    /// Password (hashed)
    #[geekorm(password)]
    pub password: String,

    /// User Role
    #[geekorm(not_null)]
    pub role: UserRole,

    /// User Session
    #[geekorm(foreign_key = "Sessions.id")]
    pub sessions: ForeignKey<i32, Sessions>,

    /// Created At
    #[geekorm(new = "chrono::Utc::now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last Login
    #[geekorm(new = "chrono::Utc::now()")]
    pub last_login: chrono::DateTime<chrono::Utc>,
}

/// User Roles Model
#[derive(Data, Debug, Default, Clone, PartialEq)]
pub enum UserRole {
    /// Admin Role
    Admin,
    /// User Role
    #[default]
    User,
    /// Agent Role
    Agent,
}

/// User State
#[derive(Data, Debug, Default, Clone, PartialEq, Eq)]
pub enum UserState {
    /// Active User
    #[default]
    Active,
    /// Reset User
    Reset,
    /// Disabled
    Disabled,
}

impl Users {
    /// Create a new user
    pub async fn create<'a, T>(
        connection: &'a T,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut session = Sessions::new(SessionType::User, SessionState::Active);
        session.save(connection).await?;

        let mut user = Users::new(username, password, UserRole::User, session.id);
        user.save(connection).await?;
        user.fetch_sessions(connection).await?;
        Ok(user)
    }

    /// User Login function
    pub async fn login<'a, T>(
        connection: &'a T,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<(Self, Sessions), KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let username = username.into();
        let password = password.into();
        let mut user = match Users::fetch_by_username(connection, username).await {
            Ok(u) => u,
            Err(e) => {
                log::warn!("Failed to login due to error: {}", e);
                return Err(KonarrError::AuthenticationError(
                    "Invalid credentials".to_string(),
                ));
            }
        };

        if user.state == UserState::Disabled {
            return Err(KonarrError::Unauthorized);
        }

        if !user.check_password(password)? {
            Err(KonarrError::AuthenticationError(
                "Invalid credentials".to_string(),
            ))
        } else {
            log::info!("Logging in user: {:?}", user.id);
            let login_time = chrono::Utc::now();

            let mut session = user.fetch_sessions(connection).await?;
            session.state = SessionState::Active;
            session.regenerate_token();
            session.last_accessed = login_time;
            session.update(connection).await?;

            log::info!("Created new session for user");
            user.last_login = login_time;
            user.update(connection).await?;

            Ok((user, session))
        }
    }

    /// Revoke the current session of the user
    pub async fn logout<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        self.sessions.data.state = SessionState::Inactive;
        self.sessions.data.update(connection).await?;
        log::info!("Logged out user :: {:?}", self.id);
        Ok(())
    }

    /// Validate Users Session
    pub fn validate_session(&self, config: &SessionsConfig) -> bool {
        let config = self.get_config(config);

        // is session active?
        if self.sessions.data.state == SessionState::Inactive {
            return false;
        }

        let now = chrono::Utc::now();
        let delta = chrono::TimeDelta::hours(config.expires.into());

        let deltaresult = self.sessions.data.last_accessed + delta;

        if deltaresult < now {
            return false;
        }

        true
    }

    /// Update Session Last Accessed time
    pub async fn update_access<'a, T>(&mut self, connection: &'a T) -> Result<(), KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        self.sessions.data.last_accessed = chrono::Utc::now();
        self.sessions.data.update(connection).await?;
        Ok(())
    }

    /// Get Configuration
    pub fn get_config<'c>(&self, config: &'c SessionsConfig) -> &'c SessionsRoleConfig {
        // Admin / User
        match self.role {
            UserRole::Admin => &config.admins,
            UserRole::User => &config.users,
            UserRole::Agent => &config.agents,
        }
    }

    /// Count Active Users
    pub async fn count_active<'a, T>(connection: &'a T) -> Result<i64, geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Users::row_count(
            connection,
            Users::query_count()
                .where_eq("state", UserState::Active)
                .build()?,
        )
        .await
    }

    /// Count Inactive Users
    pub async fn count_inactive<'a, T>(connection: &'a T) -> Result<i64, geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Users::row_count(
            connection,
            Users::query_count()
                .where_eq("state", UserState::Disabled)
                .build()?,
        )
        .await
    }
}

// impl From<&str> for UserState {
//     fn from(value: &str) -> Self {
//         match value.to_lowercase().as_str() {
//             "active" | "activate" => UserState::Active,
//             "reset" => UserState::Reset,
//             _ => UserState::Disabled,
//         }
//     }
// }

// impl ToString for UserState {
//     fn to_string(&self) -> String {
//         match self {
//             UserState::Active => "active".to_string(),
//             UserState::Reset => "reset".to_string(),
//             UserState::Disabled => "disabled".to_string(),
//         }
//     }
// }

// impl From<&str> for UserRole {
//     fn from(role: &str) -> Self {
//         match role {
//             "admin" => UserRole::Admin,
//             "agent" => UserRole::Agent,
//             _ => UserRole::User,
//         }
//     }
// }

// impl ToString for UserRole {
//     fn to_string(&self) -> String {
//         match self {
//             UserRole::Admin => "admin".to_string(),
//             UserRole::User => "user".to_string(),
//             UserRole::Agent => "agent".to_string(),
//         }
//     }
// }
