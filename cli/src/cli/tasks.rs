use clap::Subcommand;
use konarr::{
    tasks::{advisories::scan_projects, alert_calculator, catalogue},
    utils::grypedb::GrypeDatabase,
    Config,
};
use log::{debug, info};

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
                let mut grype_conn = GrypeDatabase::connect(&grype_path).await?;
                grype_conn.fetch_vulnerabilities().await?;

                scan_projects(&connection, &grype_conn).await?;
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
