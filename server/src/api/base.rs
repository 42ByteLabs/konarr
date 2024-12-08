use konarr::{
    models::settings::{find_statistic, keys::Setting, ServerSettings},
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
    pub total: u64,
    pub servers: u64,
    pub containers: u64,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct DependenciesSummary {
    pub total: u64,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct SecuritySummary {
    pub advisories: u64,
    pub total: u64,
    pub critical: u64,
    pub high: u64,
    pub medium: u64,
    pub low: u64,
    pub informational: u64,
    pub malware: u64,
    pub unmaintained: u64,
    pub unknown: u64,
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

    let init: bool = ServerSettings::fetch_by_name(&connection, Setting::Initialized)
        .await?
        .boolean();
    let registration: bool = ServerSettings::fetch_by_name(&connection, Setting::Registration)
        .await?
        .boolean();

    if let Some(session) = &session {
        let stats = ServerSettings::fetch_statistics(&connection).await?;

        let security: Option<SecuritySummary> =
            if ServerSettings::get_bool(&connection, Setting::Security).await? {
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
                total: find_statistic(&stats, Setting::StatsProjectsTotal),
                containers: find_statistic(&stats, Setting::StatsProjectsContainers),
                servers: find_statistic(&stats, Setting::StatsProjectsServers),
                ..Default::default()
            }),
            dependencies: Some(DependenciesSummary {
                total: find_statistic(&stats, Setting::StatsDependenciesTotal),
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
            if setting.name == Setting::SecurityAlertsTotal {
                summary.total = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsCritical {
                summary.critical = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsHigh {
                summary.high = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsMedium {
                summary.medium = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsLow {
                summary.low = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsInformational {
                summary.informational = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsMalware {
                summary.malware = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsUnmaintained {
                summary.unmaintained = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::SecurityAlertsUnknown {
                summary.unknown = setting.value.parse().unwrap_or(0);
            }
        }

        summary
    }
}
