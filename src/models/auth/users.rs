//! # Users Models
use geekorm::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    utils::config::{SessionsConfig, SessionsRoleConfig},
    KonarrError,
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
#[derive(Data, Debug, Default, Clone)]
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

    /// Revoke the current session of the user
    pub async fn logout<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        self.sessions.data.state = SessionState::Inactive;
        self.sessions.data.update(connection).await
    }

    /// Validate Users Session
    pub async fn validate_session<'a, T>(&self, _connection: &'a T, config: &SessionsConfig) -> bool
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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

impl ToString for UserState {
    fn to_string(&self) -> String {
        match self {
            UserState::Active => "active".to_string(),
            UserState::Reset => "reset".to_string(),
            UserState::Disabled => "disabled".to_string(),
        }
    }
}

impl From<&str> for UserRole {
    fn from(role: &str) -> Self {
        match role {
            "admin" => UserRole::Admin,
            "agent" => UserRole::Agent,
            _ => UserRole::User,
        }
    }
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::Admin => "admin".to_string(),
            UserRole::User => "user".to_string(),
            UserRole::Agent => "agent".to_string(),
        }
    }
}
