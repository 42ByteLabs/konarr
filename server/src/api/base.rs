use geekorm::prelude::*;
use konarr::{
    models::{settings::ServerSettings, Component, Projects},
    KONARR_VERSION,
};
use rocket::{serde::json::Json, State};

use crate::{guards::Session, AppState};

use super::ApiResult;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct BaseResponse {
    pub version: String,
    pub commit: String,
    pub config: ConfigResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<ProjectsSummary>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DependenciesSummary>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySummary>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct ConfigResponse {
    pub initialised: bool,
    pub registration: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct UserResponse {
    pub username: String,
    pub avatar: Option<String>,
    pub role: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct ProjectsSummary {
    pub total: u32,
    pub servers: u32,
    pub containers: u32,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct DependenciesSummary {
    pub total: u32,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct SecuritySummary {
    pub advisories: u32,
    pub total: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub informational: u32,
    pub malware: u32,
    pub unmaintained: u32,
    pub unknown: u32,
}

impl Default for BaseResponse {
    fn default() -> Self {
        let commit = env!("KONARR_GIT_COMMIT").to_string();
        Self {
            version: KONARR_VERSION.to_string(),
            commit,
            config: ConfigResponse {
                initialised: true,
                registration: true,
            },
            user: None,
            projects: None,
            dependencies: None,
            security: None,
        }
    }
}

#[get("/")]
pub async fn base(state: &State<AppState>, session: Option<Session>) -> ApiResult<BaseResponse> {
    let connection = state.db.connect()?;

    let init: bool = ServerSettings::fetch_by_name(&connection, "initialized")
        .await?
        .boolean();
    let registration: bool = ServerSettings::fetch_by_name(&connection, "registration")
        .await?
        .boolean();

    if let Some(session) = &session {
        let dependencies_total =
            Component::row_count(&connection, Component::query_count().build()?).await? as u32;

        let projects_total = Projects::count_active(&connection).await? as u32;

        let security: Option<SecuritySummary> =
            if ServerSettings::get_bool(&connection, "security").await? {
                let security_counts =
                    ServerSettings::get_namespace(&connection, "security.alerts").await?;

                Some(SecuritySummary::from(security_counts))
            } else {
                None
            };

        Ok(Json(BaseResponse {
            config: ConfigResponse {
                initialised: !init,
                registration,
            },
            user: Some(UserResponse {
                username: session.user.username.clone(),
                avatar: None,
                role: session.user.role.to_string(),
            }),
            projects: Some(ProjectsSummary {
                total: projects_total,
                containers: Projects::count_containers(&connection).await? as u32,
                servers: Projects::count_servers(&connection).await? as u32,
                ..Default::default()
            }),
            dependencies: Some(DependenciesSummary {
                total: dependencies_total,
                ..Default::default()
            }),
            security,
            ..Default::default()
        }))
    } else {
        info!("No Active Session");
        Ok(Json(BaseResponse {
            config: ConfigResponse {
                initialised: !init,
                registration,
            },
            ..Default::default()
        }))
    }
}

impl From<Vec<ServerSettings>> for SecuritySummary {
    fn from(value: Vec<ServerSettings>) -> Self {
        let mut summary = SecuritySummary::default();

        for setting in value.iter() {
            if setting.name.as_str() == "security.alerts.total" {
                summary.total = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.critical" {
                summary.critical = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.high" {
                summary.high = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.medium" {
                summary.medium = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.low" {
                summary.low = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.informational" {
                summary.informational = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.malware" {
                summary.malware = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.unmaintained" {
                summary.unmaintained = setting.value.parse().unwrap_or(0);
            } else if setting.name.as_str() == "security.alerts.unknown" {
                summary.unknown = setting.value.parse().unwrap_or(0);
            }
        }

        summary
    }
}
