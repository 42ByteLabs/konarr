use anyhow::Result;
use konarr::Config;
use log::{debug, info};

pub async fn statistics(config: &Config) -> Result<()> {
    #[cfg(feature = "database")]
    {
        database_statistics(config).await?;
    }
    #[cfg(not(feature = "database"))]
    {
        server_statistics(config).await?;
    }

    Ok(())
}

#[cfg(feature = "database")]
async fn database_statistics(config: &Config) -> Result<()> {
    let connection = &config.database().await?.connect()?;

    konarr::tasks::statistics(connection).await?;

    info!("Database Statistics");
    let statistics = konarr::models::settings::ServerSettings::fetch_statistics(connection).await?;

    print_stats(
        "Projects Statistics",
        vec![
            ("Projects", find_stat("stats.projects.total", &statistics)),
            ("Servers", find_stat("stats.projects.servers", &statistics)),
            (
                "Containers",
                find_stat("stats.projects.containers", &statistics),
            ),
        ],
    );

    print_stats(
        "Dependencies",
        vec![
            ("Total", find_stat("stats.dependencies.total", &statistics)),
            (
                "Libraries",
                find_stat("stats.dependencies.libraries", &statistics),
            ),
            (
                "Frameworks",
                find_stat("stats.dependencies.frameworks", &statistics),
            ),
            (
                "Operating Systems",
                find_stat("stats.dependencies.operating-systems", &statistics),
            ),
            (
                "Languages",
                find_stat("stats.dependencies.languages", &statistics),
            ),
            (
                "Package Managers",
                find_stat("stats.dependencies.package-managers", &statistics),
            ),
            (
                "Compression Libraries",
                find_stat("stats.dependencies.compression-libraries", &statistics),
            ),
            (
                "Cryptographic Libraries",
                find_stat("stats.dependencies.cryptographic-libraries", &statistics),
            ),
            (
                "Databases",
                find_stat("stats.dependencies.databases", &statistics),
            ),
            (
                "Operating Environments",
                find_stat("stats.dependencies.operating-environments", &statistics),
            ),
            (
                "Middleware",
                find_stat("stats.dependencies.middleware", &statistics),
            ),
        ],
    );

    print_stats(
        "Security Statistics",
        vec![
            ("Total", find_stat("security.alerts.total", &statistics)),
            (
                "Critical",
                find_stat("security.alerts.critical", &statistics),
            ),
            ("High", find_stat("security.alerts.high", &statistics)),
            ("Medium", find_stat("security.alerts.medium", &statistics)),
            ("Low", find_stat("security.alerts.low", &statistics)),
            (
                "Informational",
                find_stat("security.alerts.informational", &statistics),
            ),
            ("Malware", find_stat("security.alerts.malware", &statistics)),
            (
                "Unmaintained",
                find_stat("security.alerts.unmaintained", &statistics),
            ),
            ("Unknown", find_stat("security.alerts.unknown", &statistics)),
        ],
    );

    Ok(())
}

async fn server_statistics(config: &Config) -> Result<()> {
    debug!("Server Statistics");
    let (_client, serverinfo) = crate::client(&config).await?;
    // Check if the user is authenticated
    if !serverinfo.user.is_some() {
        info!("User is not authenticated");
    } else {
        info!("User is authenticated!");
    }
    if let Some(psummary) = serverinfo.projects {
        print_stats(
            "Projects Statistics",
            vec![
                ("Projects", psummary.total),
                ("Servers", psummary.servers),
                ("Containers", psummary.containers),
            ],
        );
    }
    if let Some(dsummary) = serverinfo.dependencies {
        print_stats(
            "Dependency Statistics",
            vec![
                ("Total", dsummary.total),
                ("Libraries", dsummary.libraries),
                ("Frameworks", dsummary.frameworks),
                ("Operating Systems", dsummary.operating_systems),
                ("Languages", dsummary.languages),
                ("Package Managers", dsummary.package_managers),
                ("Compression Libraries", dsummary.compression_libraries),
                ("Cryptographic Libraries", dsummary.cryptographic_libraries),
                ("Databases", dsummary.databases),
                ("Operating Environments", dsummary.operating_environments),
                ("Middleware", dsummary.middleware),
            ],
        );
    }
    if let Some(security) = serverinfo.security {
        print_stats(
            "Security Statistics",
            vec![
                ("Critical", security.critical),
                ("High", security.high),
                ("Medium", security.medium),
                ("Low", security.low),
                ("Informational", security.informational),
                ("Malware", security.malware),
                ("Unmaintained", security.unmaintained),
                ("Unknown", security.unknown),
            ],
        );
    }
    // info!("Dependencies :: {}", serverinfo.dependencies.total);

    if let Some(agent_settings) = serverinfo.agent {
        info!("----- {:^26} -----", "Agent Settings");
        let tools = konarr::tools::ToolConfig::tools().await?;
        let tool_available = if tools
            .iter()
            .find(|t| t.name == agent_settings.tool.to_lowercase())
            .is_some()
        {
            "✅"
        } else {
            "❌"
        };

        info!("Agent settings");
        info!(
            " > {} Tool to use: {} ",
            tool_available, agent_settings.tool
        );

        info!("Other tools available:");
        for tool in tools.iter() {
            if !tool.version.is_empty() {
                info!(" > {} (v{})", tool.name, tool.version);
            } else {
                info!(" > {}", tool.name);
            }
        }
    }

    Ok(())
}

fn print_stats(title: &str, stats: Vec<(&str, u32)>) {
    info!("----- {:^26} -----", title);
    for (name, value) in stats.iter() {
        let emoji = find_emoji(name).unwrap_or("❓");
        info!(" > {} {:<24}: {}", emoji, name, value);
    }
}

fn find_emoji(name: &str) -> Option<&str> {
    EMOJIS.iter().find(|(_e, n)| *n == name).map(|(e, _)| *e)
}

#[cfg(feature = "database")]
fn find_stat(name: &str, settings: &Vec<konarr::models::settings::ServerSettings>) -> u32 {
    settings
        .iter()
        .find(|s| s.name.to_string() == name)
        .map(|s| s.value.parse().unwrap_or(0))
        .unwrap_or(0)
}

const EMOJIS: [(&str, &str); 22] = [
    ("⚡", "Projects"),
    ("💻", "Servers"),
    ("📦", "Containers"),
    ("📦", "Libraries"),
    ("📦", "Frameworks"),
    ("🖥️ ", "Operating Systems"),
    ("📝", "Languages"),
    ("📦", "Package Managers"),
    ("⚡", "Compression Libraries"),
    ("🔒", "Cryptographic Libraries"),
    ("🐐", "Databases"),
    ("🛞", "Operating Environments"),
    ("🔍", "Middleware"),
    ("🔍", "Total"),
    ("🔴", "Critical"),
    ("🟠", "High"),
    ("🟡", "Medium"),
    ("🟢", "Low"),
    ("ℹ️ ", "Informational"),
    ("🦠", "Malware"),
    ("🛡️ ", "Unmaintained"),
    ("❓", "Unknown"),
];
