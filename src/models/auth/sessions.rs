//! # Sessions

use geekorm::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::config::SessionsRoleConfig;

/// User Session Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sessions {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKeyInteger,

    /// Session Type
    pub session_type: SessionType,

    /// Session State
    pub state: SessionState,

    /// Token
    #[geekorm(unique, rand, rand_length = 42, rand_prefix = "konarr")]
    pub token: String,

    /// Time of Created
    #[geekorm(new = "chrono::Utc::now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last valid access of the account
    #[geekorm(new = "chrono::Utc::now()")]
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

impl Sessions {
    /// Validates the session is active and
    pub fn validate(&self, config: &SessionsRoleConfig) -> bool {
        let now = chrono::Utc::now();
        let delta = chrono::TimeDelta::hours(config.expires.into());

        let deltaresult = self.last_accessed + delta;

        self.state == SessionState::Active && deltaresult < now
    }
}

/// Session State
#[derive(Data, Debug, Default, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Active
    #[default]
    Active,
    /// Inactive
    Inactive,
}

/// Session Type
#[derive(Data, Debug, Default, Clone, PartialEq, Eq)]
pub enum SessionType {
    /// User Session
    #[default]
    User,
    /// Application Session
    Application,
}

impl From<&str> for SessionType {
    fn from(session_type: &str) -> Self {
        match session_type {
            "application" => SessionType::Application,
            _ => SessionType::User,
        }
    }
}
