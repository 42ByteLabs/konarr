use clap::Subcommand;
use konarr::{
    tasks::{advisories::scan_projects, alert_calculator},
    utils::grypedb::GrypeDatabase,
    Config,
};
use log::{debug, info};

#[derive(Subcommand, Debug, Clone)]
pub enum TaskCommands {
    /// Run the Alert Calculator
    Alerts {},
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

            info!("Completed!");
        }
        Some(TaskCommands::Grype { alerts }) => {
            info!("Running Grype Sync Task");

            let grype_path = config.data_path()?.join("grypedb");
            debug!("Grype DB Path: {:?}", grype_path);

            GrypeDatabase::sync(&grype_path).await?;

            if alerts {
                info!("Running Grype Alerts Task");
                let grype_conn = GrypeDatabase::connect(&grype_path).await?;

                scan_projects(&connection, &grype_conn).await?;
            }
        }
        None => {
            info!("No subcommand provided, running interactive mode");
        }
    }

    Ok(())
}
