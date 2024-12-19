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
    /// Version of Konarr
    pub version: String,
    /// Commit SHA of the current build
    pub commit: String,
    /// Base configuration
    pub config: ConfigResponse,
    /// Current User
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserResponse>,
    /// Projects Summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<ProjectsSummary>,
    /// Dependencies Summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<DependenciesSummary>,
    /// Security Summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySummary>,
    /// Agent Settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentResponse>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct ConfigResponse {
    /// Is the server initialised
    pub initialised: bool,
    /// Is the server open for registration
    pub registration: bool,
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
    pub libraries: u64,
    pub frameworks: u64,
    #[serde(rename = "operating-systems")]
    pub operating_systems: u64,
    pub languages: u64,
    #[serde(rename = "package-managers")]
    pub package_managers: u64,
    #[serde(rename = "compression-libraries")]
    pub compression_libraries: u64,
    #[serde(rename = "cryptographic-libraries")]
    pub cryptographic_libraries: u64,
    pub databases: u64,
    #[serde(rename = "operating-environments")]
    pub operating_environments: u64,
    pub middleware: u64,
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", crate = "rocket::serde")]
pub struct AgentResponse {
    /// Tool name
    pub tool: AgentTool,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum AgentTool {
    /// Syft
    #[default]
    Syft,
    /// Grype
    Grype,
    /// Trivy
    Trivy,
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
            agent: None,
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

        let agent: Option<AgentResponse> = if session.user.username == "konarr-agent" {
            Some(AgentResponse {
                tool: AgentTool::from(
                    ServerSettings::fetch_by_name(&connection, Setting::SecurityToolsName)
                        .await?
                        .value,
                ),
            })
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
            dependencies: Some(DependenciesSummary::from(stats)),
            security,
            agent,
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

impl From<Vec<ServerSettings>> for DependenciesSummary {
    fn from(value: Vec<ServerSettings>) -> Self {
        let mut summary = DependenciesSummary::default();

        for setting in value.iter() {
            if setting.name == Setting::StatsDependenciesTotal {
                summary.total = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsLibraries {
                summary.libraries = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsFrameworks {
                summary.frameworks = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsOperatingSystems {
                summary.operating_systems = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsLanguages {
                summary.languages = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsPackageManagers {
                summary.package_managers = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsCompressionLibraries {
                summary.compression_libraries = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsCryptographicLibraries {
                summary.cryptographic_libraries = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsDatabases {
                summary.databases = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsOperatingEnvironments {
                summary.operating_environments = setting.value.parse().unwrap_or(0);
            } else if setting.name == Setting::StatsMiddleware {
                summary.middleware = setting.value.parse().unwrap_or(0);
            }
        }

        summary
    }
}

impl From<String> for AgentTool {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "grype" => Self::Grype,
            "trivy" => Self::Trivy,
            _ => Self::Syft,
        }
    }
}
