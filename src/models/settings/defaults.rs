//! # Server Settings Defaults
use super::{Setting, SettingType};

/// Server Settings Defaults
pub const SERVER_SETTINGS_DEFAULTS: [(Setting, SettingType, &str); 39] = [
    // Registration Settings
    (Setting::Registration, SettingType::Toggle, "enabled"),
    // If we are already initialized
    (Setting::Initialized, SettingType::Boolean, "false"),
    // Server Settings
    (
        Setting::ServerUrl,
        SettingType::String,
        "http://localhost:8000",
    ),
    (
        Setting::ServerFrontendPath,
        SettingType::String,
        "/app/dist",
    ),
    // Session Settings
    (Setting::SessionAdminsExpires, SettingType::String, "1"),
    (Setting::SessionUsersExpires, SettingType::String, "24"),
    (Setting::SessionAgentsExpires, SettingType::String, "360"),
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
