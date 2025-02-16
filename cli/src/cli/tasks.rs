use clap::Subcommand;
use geekorm::prelude::*;
use konarr::{
    models::Projects,
    tasks::{alert_calculator, catalogue},
    tools::Tool,
    utils::grypedb::GrypeDatabase,
    Config,
};
use log::info;

#[derive(Subcommand, Debug, Clone)]
pub enum TaskCommands {
    /// Run the Alert Calculator
    Alerts {},
    /// Run the Catalogue Sync Task
    Catalogue {
        #[clap(short, long, default_value = "false")]
        force: bool,
    },
    /// Run the Grype Sync Task
    Grype {
        /// Run the Grype Alerts Tas
        #[clap(short, long, default_value = "false")]
        alerts: bool,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<TaskCommands>,
) -> Result<(), konarr::KonarrError> {
    let connection = config.database().await?.connect()?;

    match subcommands {
        Some(TaskCommands::Alerts {}) => {
            alert_calculator(&connection).await?;
        }
        Some(TaskCommands::Catalogue { force }) => {
            catalogue(&connection, force).await?;
        }
        Some(TaskCommands::Grype { alerts }) => {
            info!("Running Grype Sync Task");

            let grype_path = config.data_path()?.join("grypedb");
            info!("Grype data path: {:?}", grype_path);

            GrypeDatabase::sync(&grype_path).await?;

            if alerts {
                info!("Running Grype Alerts Task");

                let projects = Projects::all(&connection).await?;
                let mut snaps = vec![];
                for proj in projects.iter() {
                    if let Some(snap) = proj.fetch_latest_snapshot(&connection).await? {
                        snaps.push(snap);
                    }
                }

                let tool_config = konarr::tools::grype::Grype::init().await;

                for snap in snaps {
                    let mut proj = snap.fetch_project(&connection).await?;
                    info!("Scanning: {}", proj.name);

                    konarr::tasks::advisories::scan_project(
                        &config,
                        &connection,
                        &tool_config,
                        &mut proj,
                    )
                    .await?;
                }

                // scan_projects(&config, &connection).await?;
                info!("Grype Alerts Task Complete");
            }

            konarr::tasks::alert_calculator(&connection).await?;
        }
        None => {
            info!("No subcommand provided, running interactive mode");
        }
    }
    info!("Completed!");

    Ok(())
}
