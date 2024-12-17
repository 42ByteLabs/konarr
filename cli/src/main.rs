#![deny(unsafe_code)]

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};

mod cli;
mod utils;

use cli::init;
use konarr::{
    bom::{BomParser, Parsers},
    client::snapshot::KonarrSnapshot,
    Config,
};
use utils::interactive::{prompt_input, prompt_password};

async fn client(config: &Config) -> Result<(konarr::KonarrClient, konarr::client::ServerInfo)> {
    let client = if let Some(token) = &config.agent.token {
        config.server.client_with_token(token.to_string())?
    } else {
        debug!("Interactively logging in");
        let username = prompt_input("Username:")?;
        let password = prompt_password("Password:")?;

        let mut client = config
            .server
            .client_with_credentials(username, password)
            .expect("Could not create client");
        client
            .login()
            .await
            .map_err(|e| anyhow!("Failed to login with credentials: {}", e))?;
        info!("Logged in successfully with credentials");
        client
    };

    let serverinfo = client.server().await?;
    info!("Server - v{} - '{}'", serverinfo.version, client.url());
    info!("Client - v{}", client.version());

    if client.version() != serverinfo.version {
        warn!(
            "Client version ({}) does not match server version ({})",
            client.version(),
            serverinfo.version
        );
    }

    Ok((client, serverinfo))
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = init();

    #[allow(unused_mut)]
    let mut config = match Config::load(&arguments.config) {
        Ok(config) => config,
        Err(error) => {
            warn!("Failed to load configuration: {}", error);
            Config::default()
        }
    };
    if let Some(instance) = arguments.instance {
        config.server.set_instance(&instance)?;
    }

    match arguments.commands {
        Some(cli::ArgumentCommands::Agent { docker_socket }) => {
            // HACK: Manually set some stuff for now

            config.agent.docker_socket = docker_socket;
            config.agent.project_id = arguments.project_id;
            config.agent.create = arguments.auto_create;
            config.agent.host = arguments.hostname;

            if let Some(token) = arguments.agent_token {
                config.agent.token = Some(token);
            }

            let (client, serverinfo) = client(&config).await?;

            // Check if the user is authenticated
            if !serverinfo.user.is_some() {
                error!("User is not authenticated");
                return Ok(());
            }
            info!(
                "Logged into server as: {}",
                serverinfo.user.unwrap().username
            );

            Ok(cli::agent::setup(&config, &client).await?)
        }
        Some(cli::ArgumentCommands::Scan {
            image,
            list,
            tool,
            output,
        }) => {
            if list {
                let tools = konarr::tools::get_available_tools().await?;
                info!("Available tools:");
                for tool in tools {
                    info!("> {}", tool);
                }
                return Ok(());
            }

            if let Some(image) = image {
                config.agent.tool = tool;
                let result = konarr::tools::run(&config, image).await?;

                if let Some(output) = output {
                    info!("Writing output to: {}", output);
                    std::fs::write(output, result)?;
                } else {
                    let parse = Parsers::parse(&result.as_bytes())?;
                    log::info!("SBOM Summary:");
                    log::info!(" > Dependencies     : {}", parse.components.len());
                    log::info!(" > Vulnerabilities  : {}", parse.vulnerabilities.len());
                }
            } else {
                return Err(anyhow!("No image provided"));
            }

            Ok(())
        }
        Some(cli::ArgumentCommands::UploadSbom { input, snapshot_id }) => {
            if !input.exists() || !input.is_file() {
                return Err(anyhow!("Input file does not exist or is not a file"));
            }
            let (client, serverinfo) = client(&config).await?;

            if !serverinfo.user.is_some() {
                error!("User is not authenticated");
                return Ok(());
            }
            info!(
                "Logged into server as: {}",
                serverinfo.user.unwrap().username
            );

            let snapshot = if let Some(snapshot_id) = snapshot_id {
                KonarrSnapshot::by_id(&client, snapshot_id).await?
            } else {
                let project_id = if let Some(projid) = arguments.project_id {
                    projid
                } else {
                    prompt_input("Project ID")?.parse()?
                };
                info!("Uploading SBOM to project: {}", project_id);
                let snapshot: KonarrSnapshot = KonarrSnapshot::create(&client, project_id).await?;
                info!("Created snapshot: {}", snapshot.id);
                snapshot
            };

            info!("Uploading SBOM...");
            let data = std::fs::read(&input)?;
            match Parsers::parse(&data) {
                Ok(bom) => {
                    info!("Validate SBOM spec supported by Konarr: {}", bom.sbom_type);
                }
                Err(e) => {
                    return Err(anyhow!("Failed to parse SBOM: {}", e));
                }
            }

            let json_data: serde_json::Value = serde_json::from_slice(&data)?;
            debug!("Serialized BOM to JSON");
            snapshot.upload_bom(&client, json_data).await?;

            Ok(())
        }
        #[cfg(feature = "database")]
        Some(cli::ArgumentCommands::Database { subcommands }) => {
            if let Some(url) = arguments.database_url {
                config.database.path = Some(url.into());
            }
            cli::database::run(&config, subcommands).await
        }
        #[cfg(feature = "database")]
        Some(cli::ArgumentCommands::Display { subcommands }) => {
            Ok(cli::display::run(&config, subcommands).await?)
        }
        #[cfg(feature = "database")]
        Some(cli::ArgumentCommands::Index { subcommands }) => {
            Ok(cli::index::run(&config, subcommands).await?)
        }
        #[cfg(feature = "database")]
        Some(cli::ArgumentCommands::Search { subcommands }) => {
            Ok(cli::search::run(&config, subcommands).await?)
        }
        #[cfg(feature = "tasks")]
        Some(cli::ArgumentCommands::Tasks { subcommands }) => {
            Ok(cli::tasks::run(&config, subcommands).await?)
        }
        None => {
            info!("No command provided, showing server info");
            let (_client, serverinfo) = client(&config).await?;

            // Check if the user is authenticated
            if !serverinfo.user.is_some() {
                info!("User is not authenticated");
            } else {
                info!("User is authenticated!");
            }

            if let Some(psummary) = serverinfo.projects {
                info!("----- Project Statistics -----");
                info!(" > ⚡ Projects: {}", psummary.total);
                info!(" > 💻 Servers: {}", psummary.servers);
                info!(" > 📦 Containers: {}", psummary.containers);
            }
            if let Some(dsummary) = serverinfo.dependencies {
                info!("----- Dependency Statistics -----");
                info!(" > 🧰 Dependencies: {}", dsummary.total);
            }
            if let Some(security) = serverinfo.security {
                info!("----- Security Summary -----");
                info!(" > Total: {}", security.total);
                info!(" > 🔴 Critical: {}", security.critical);
                info!(" > 🟠 High: {}", security.high);
                info!(" > 🟡 Medium: {}", security.medium);
                info!(" > 🟢 Low: {}", security.low);
                info!(" > ℹ️  Informational: {}", security.informational);
                info!(" > 🦠 Malware: {}", security.malware);
                info!(" > 🛡️ Unmaintained: {}", security.unmaintained);
                info!(" > ❓ Unknown: {}", security.unknown);
            }
            // info!("Dependencies :: {}", serverinfo.dependencies.total);

            if let Some(agent_settings) = serverinfo.agent {
                info!("----- Agent Settings -----");
                let tools = konarr::tools::get_available_tools().await?;
                let tool_available = if tools.contains(&agent_settings.tool.to_lowercase()) {
                    "✅"
                } else {
                    "❌"
                };

                info!("Agent settings");
                info!(
                    " > {} Tool to use: {} ",
                    tool_available, agent_settings.tool
                );
            }

            Ok(())
        }
    }
}
