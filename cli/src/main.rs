use anyhow::{anyhow, Result};

mod cli;
mod utils;

use cli::{init, Arguments};
use konarr::{client::projects::KonarrProjects, Config};
use log::{error, info, warn};
use tokio::spawn;
use tokio_schedule::{every, Job};

async fn run(arguments: Arguments) -> Result<()> {
    let mut config = match Config::load(&arguments.config) {
        Ok(config) => config,
        Err(error) => {
            warn!("Failed to load configuration: {}", error);
            Config::default()
        }
    };

    match arguments.commands {
        Some(cli::ArgumentCommands::Agent {
            agent_token,
            subcommands,
            project_id,
            hostname,
        }) => {
            let client = config
                .server
                .client_with_token(agent_token.expect("Agent Token must be provided"))?;
            info!("Client created");
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

            // ID -> Name -> New Project
            let server_project = if let Some(project_id) = project_id {
                KonarrProjects::by_id(&client, project_id).await?
            } else if let Some(config_project_id) = config.project.id {
                KonarrProjects::by_id(&client, config_project_id).await?
            } else if let Some(hostname) = &hostname {
                match KonarrProjects::by_name(&client, hostname).await {
                    Ok(project) => project,
                    Err(error) => {
                        error!("Failed to get project by name: {}", error);
                        return Err(error.into());
                    }
                }
            } else {
                error!("Failed to get project by ID or Name");
                return Err(anyhow::anyhow!("Failed to get project by ID or Name"));
            };

            info!("Project :: {}", server_project.id);

            Ok(cli::agent::run(&config, subcommands, &client, server_project).await?)
        }
        #[cfg(feature = "database")]
        Some(cli::ArgumentCommands::Database {
            subcommands,
            database_url,
        }) => {
            if let Some(url) = database_url {
                config.database.path = Some(url);
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
        None => Err(anyhow!("No subcommand provided")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = init();

    if arguments.monitoring {
        info!("Monitoring mode enabled");
        let task = every(1).minutes().perform(move || {
            let arguments = arguments.clone();

            async move {
                info!("Running task...");

                run(arguments).await.expect("Panic in monitoring mode...");

                info!("Finishing task... Waiting for next");
            }
        });
        spawn(task).await?;

        Ok(())
    } else {
        run(arguments).await
    }
}
