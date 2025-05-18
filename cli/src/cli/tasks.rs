use clap::Subcommand;
use konarr::{
    Config,
    models::{Projects, dependencies::snapshots::SnapshotState},
    tasks::{
        AdvisoriesSyncTask, AdvisoriesTask, AlertCalculatorTask, CatalogueTask, TaskTrait,
        sbom::SbomTask,
    },
    tools::Tool,
    utils::grypedb::GrypeDatabase,
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
    Sbom {
        /// State of the SBOM
        #[clap(short, long, default_value = "Processing")]
        state: String,
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
    let database = config.database().await?;

    match subcommands {
        Some(TaskCommands::Alerts {}) => {
            AlertCalculatorTask::spawn(&database).await?;
        }
        Some(TaskCommands::Catalogue { force }) => {
            CatalogueTask::spawn(&database).await?;
        }
        Some(TaskCommands::Sbom { state }) => {
            let task = SbomTask::sbom_by_state(&state);
            info!("Running SBOM Task with state: {}", state);

            task.run(&database).await?;
        }
        Some(TaskCommands::Grype { alerts }) => {
            info!("Running Grype Sync Task");

            AdvisoriesSyncTask::spawn(&database).await?;

            AdvisoriesTask::spawn(&database).await?;
            // scan_projects(&config, &connection).await?;
            info!("Grype Alerts Task Complete");

            AlertCalculatorTask::spawn(&database).await?;
        }
        None => {
            info!("No subcommand provided, running interactive mode");
        }
    }
    info!("Completed!");

    Ok(())
}
