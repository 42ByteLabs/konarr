//! # Settings Keys

use geekorm::prelude::*;

use super::SettingType;

#[derive(Data, Debug, Default, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Setting {
    // Setup Settings
    #[geekorm(key = "initialized")]
    Initialized,
    // Registration
    #[geekorm(key = "registration")]
    Registration,
    // Agent Settings
    #[geekorm(key = "agent")]
    Agent,
    #[geekorm(key = "agent.key")]
    AgentKey,
    #[geekorm(key = "agent.tool")]
    AgentTool,
    #[geekorm(key = "agent.tool.auto-install")]
    AgentToolAutoInstall,
    #[geekorm(key = "agent.tool.auto-update")]
    AgentToolAutoUpdate,

    // Statistics - Projects
    #[geekorm(key = "stats.projects.total")]
    StatsProjectsTotal,
    #[geekorm(key = "stats.projects.active")]
    StatsProjectsActive,
    #[geekorm(key = "stats.projects.inactive")]
    StatsProjectsInactive,
    #[geekorm(key = "stats.projects.archived")]
    StatsProjectsArchived,
    #[geekorm(key = "stats.projects.servers")]
    StatsProjectsServers,
    #[geekorm(key = "stats.projects.groups")]
    StatsProjectsGroups,
    #[geekorm(key = "stats.projects.containers")]
    StatsProjectsContainers,

    // Statistics - Security
    #[geekorm(key = "security.alerts.total")]
    SecurityAlertsTotal,
    #[geekorm(key = "security.alerts.critical")]
    SecurityAlertsCritical,
    #[geekorm(key = "security.alerts.high")]
    SecurityAlertsHigh,
    #[geekorm(key = "security.alerts.medium")]
    SecurityAlertsMedium,
    #[geekorm(key = "security.alerts.low")]
    SecurityAlertsLow,
    #[geekorm(key = "security.alerts.informational")]
    SecurityAlertsInformational,
    #[geekorm(key = "security.alerts.unmaintained")]
    SecurityAlertsUnmaintained,
    #[geekorm(key = "security.alerts.malware")]
    SecurityAlertsMalware,
    #[geekorm(key = "security.alerts.unknown")]
    SecurityAlertsUnknown,

    // Statistics - Users
    #[geekorm(key = "stats.users.total")]
    StatsUsersTotal,
    #[geekorm(key = "stats.users.active")]
    StatsUsersActive,
    #[geekorm(key = "stats.users.inactive")]
    StatsUsersInactive,

    // Statistics - Dependencies
    #[geekorm(key = "stats.dependencies.total")]
    StatsDependenciesTotal,
    #[geekorm(key = "stats.dependencies.libraries")]
    StatsLibraries,
    #[geekorm(key = "stats.dependencies.applications")]
    StatsApplications,
    #[geekorm(key = "stats.dependencies.frameworks")]
    StatsFrameworks,
    #[geekorm(key = "stats.dependencies.operating-systems")]
    StatsOperatingSystems,
    #[geekorm(key = "stats.dependencies.package-managers")]
    StatsPackageManagers,
    #[geekorm(key = "stats.dependencies.languages")]
    StatsLanguages,
    #[geekorm(key = "stats.dependencies.databases")]
    StatsDatabases,
    #[geekorm(key = "stats.dependencies.cryptographic-libraries")]
    StatsCryptographicLibraries,
    #[geekorm(key = "stats.dependencies.compression")]
    StatsCompressionLibraries,
    #[geekorm(key = "stats.dependencies.operating-environments")]
    StatsOperatingEnvironments,
    #[geekorm(key = "stats.dependencies.middleware")]
    StatsMiddleware,

    #[geekorm(key = "stats.dependencies.secure")]
    StatsDependenciesSecure,
    #[geekorm(key = "stats.dependencies.insecure")]
    StatsDependenciesInsecure,
    /// Unused dependencies (previously used but not anymore)
    #[geekorm(key = "stats.dependencies.unused")]
    StatsDependenciesUnused,

    // Security
    #[geekorm(key = "security")]
    Security,
    /// Security tool name to use by default
    #[geekorm(key = "security.tools.name")]
    SecurityToolsName,
    /// Allow Security tools to submit alerts
    #[geekorm(key = "security.tools.alerts")]
    SecurityToolsAlerts,

    // Security Rescan Setting
    #[geekorm(key = "security.rescan")]
    SecurityRescan,

    // Security Advisories
    #[geekorm(key = "security.advisories")]
    SecurityAdvisories,
    #[geekorm(key = "security.advisories.pull")]
    SecurityAdvisoriesPull,
    #[geekorm(key = "security.advisories.polling")]
    SecurityAdvisoriesPolling,
    #[geekorm(key = "security.advisories.version")]
    SecurityAdvisoriesVersion,
    #[geekorm(key = "security.advisories.updated")]
    SecurityAdvisoriesUpdated,

    // Deprecated
    #[geekorm(key = "security.polling")]
    SecurityPolling,
    #[geekorm(key = "security.alerts.other")]
    SecurityAlertsOther,
    #[geekorm(key = "security.grype")]
    SecurityGrype,
    /// Remove old typo
    #[geekorm(key = "security.alerts.infomational")]
    SecurityAlertsInfomational,

    // Unknown
    #[default]
    #[geekorm(key = "unknown")]
    Unknown,
}

/// List of depricated settings
pub const SERVER_SETTINGS_DEPRICATED: [Setting; 4] = [
    Setting::SecurityPolling,
    Setting::SecurityAlertsOther,
    Setting::SecurityGrype,
    Setting::SecurityAlertsInfomational,
];

/// Server Settings Defaults
pub const SERVER_SETTINGS_DEFAULTS: [(Setting, SettingType, &'static str); 34] = [
    // Registration Settings
    (Setting::Registration, SettingType::Toggle, "enabled"),
    // If we are already initialized
    (Setting::Initialized, SettingType::Boolean, "false"),
    // Agent Settings
    (Setting::Agent, SettingType::Toggle, "disabled"),
    (
        Setting::AgentToolAutoInstall,
        SettingType::Toggle,
        "disabled",
    ),
    (
        Setting::AgentToolAutoUpdate,
        SettingType::Toggle,
        "disabled",
    ),
    // Statistics
    (Setting::StatsProjectsTotal, SettingType::Statistics, "0"),
    (Setting::StatsProjectsActive, SettingType::Statistics, "0"),
    (Setting::StatsProjectsInactive, SettingType::Statistics, "0"),
    (Setting::StatsProjectsArchived, SettingType::Statistics, "0"),
    (Setting::StatsProjectsGroups, SettingType::Statistics, "0"),
    (Setting::StatsProjectsServers, SettingType::Statistics, "0"),
    (
        Setting::StatsProjectsContainers,
        SettingType::Statistics,
        "0",
    ),
    (
        Setting::StatsDependenciesTotal,
        SettingType::Statistics,
        "0",
    ),
    (Setting::StatsUsersTotal, SettingType::Statistics, "0"),
    (Setting::StatsUsersActive, SettingType::Statistics, "0"),
    (Setting::StatsUsersInactive, SettingType::Statistics, "0"),
    // Security Features
    (Setting::Security, SettingType::Toggle, "disabled"),
    (Setting::SecurityRescan, SettingType::Toggle, "disabled"),
    (Setting::SecurityToolsName, SettingType::SetString, "syft"),
    // Tools Settings
    (
        Setting::SecurityToolsAlerts,
        SettingType::Toggle,
        "disabled",
    ),
    // Advisories Settings
    (Setting::SecurityAdvisories, SettingType::Toggle, "disabled"),
    (
        Setting::SecurityAdvisoriesPull,
        SettingType::Toggle,
        "disabled",
    ),
    (
        Setting::SecurityAdvisoriesVersion,
        SettingType::String,
        "Unknown",
    ),
    (
        Setting::SecurityAdvisoriesUpdated,
        SettingType::Datetime,
        "Unknown",
    ),
    (
        Setting::SecurityAdvisoriesPolling,
        SettingType::Toggle,
        "disabled",
    ),
    (Setting::SecurityAlertsTotal, SettingType::Statistics, "0"),
    (
        Setting::SecurityAlertsCritical,
        SettingType::Statistics,
        "0",
    ),
    (Setting::SecurityAlertsHigh, SettingType::Statistics, "0"),
    (Setting::SecurityAlertsMedium, SettingType::Statistics, "0"),
    (Setting::SecurityAlertsLow, SettingType::Statistics, "0"),
    (
        Setting::SecurityAlertsInformational,
        SettingType::Statistics,
        "0",
    ),
    (
        Setting::SecurityAlertsUnmaintained,
        SettingType::Statistics,
        "0",
    ),
    (Setting::SecurityAlertsMalware, SettingType::Statistics, "0"),
    (Setting::SecurityAlertsUnknown, SettingType::Statistics, "0"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_value() {
        let key = Setting::Registration;
        assert_eq!(Value::from(key), Value::from("registration"));
        let key = Setting::Initialized;
        assert_eq!(Value::from(key), Value::from("initialized"));
        let key = Setting::Security;
        assert_eq!(Value::from(key), Value::from("security"));
        let key = Setting::StatsProjectsTotal;
        assert_eq!(Value::from(key), Value::from("stats.projects.total"));

        let column = Value::Text("registration".to_string());
        assert_eq!(Setting::from(column), Setting::Registration);
    }

    #[test]
    fn test_to_string() {
        let key = Setting::Registration;
        assert_eq!(key.to_string(), "registration");
        let key = Setting::Initialized;
        assert_eq!(key.to_string(), "initialized");
        let security = Setting::Security;
        assert_eq!(security.to_string(), "security");
        let stats = Setting::StatsProjectsTotal;
        assert_eq!(stats.to_string(), "stats.projects.total");
    }

    #[test]
    fn test_from_string() {
        let key = Setting::from("registration");
        assert_eq!(key, Setting::Registration);
        let key = Setting::from("initialized");
        assert_eq!(key, Setting::Initialized);
        let key = Setting::from("security");
        assert_eq!(key, Setting::Security);
        let key = Setting::from("stats.projects.total");
        assert_eq!(key, Setting::StatsProjectsTotal);
    }
}
