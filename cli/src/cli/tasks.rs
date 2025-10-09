use clap::Subcommand;
use konarr::{
    Config,
    tasks::{
        AdvisoriesSyncTask, AdvisoriesTask, AlertCalculatorTask, CatalogueTask, DependenciesTask,
        TaskTrait, sbom::SbomTask,
    },
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
    Depencencies {},
    Sbom {
        /// State of the SBOM
        #[clap(short, long, default_value = "Processing")]
        state: String,
    },
    /// Run the Grype Sync Task
    Grype {
        /// Run the Grype Alerts Task
        #[clap(short, long, default_value = "false")]
        alerts: bool,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<TaskCommands>,
) -> Result<(), konarr::KonarrError> {
    let database = config.database().await?;

    match subcommands {
        Some(TaskCommands::Alerts {}) => {
            AlertCalculatorTask::spawn(&database).await?;
        }
        Some(TaskCommands::Catalogue { force }) => {
            let task = if force {
                CatalogueTask::force()
            } else {
                CatalogueTask::default()
            };
            task.run(&database).await?;
        }
        Some(TaskCommands::Depencencies {}) => {
            // DependenciesTask::spawn(&database).await?;
            DependenciesTask::spawn(&database).await?;
        }
        Some(TaskCommands::Sbom { state }) => {
            let task = SbomTask::sbom_by_state(&state);
            info!("Running SBOM Task with state: {}", state);

            task.run(&database).await?;
        }
        Some(TaskCommands::Grype { alerts }) => {
            info!("Running Grype Sync Task");

            AdvisoriesSyncTask::spawn(&database).await?;

            if alerts {
                info!("Running Grype Alerts Task");
                AdvisoriesTask::spawn(&database).await?;
            }

            AlertCalculatorTask::spawn(&database).await?;
            info!("Grype Alerts Task Complete");
        }
        None => {
            info!("No subcommand provided, running interactive mode");
        }
    }
    info!("Completed!");

    Ok(())
}
