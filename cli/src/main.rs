use anyhow::{anyhow, Result};

mod cli;
mod utils;

use cli::init;
use konarr::Config;
use log::{error, info, warn};

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

            let client = config.server.client_with_token(
                config
                    .agent
                    .token
                    .clone()
                    .expect("Agent Token must be provided"),
            )?;
            info!("Client created :: {}", client.url());

            let serverinfo = client.server().await?;
            info!("Server Info :: {}", serverinfo.version);

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
        None => Err(anyhow!("No subcommand provided")),
    }
}
