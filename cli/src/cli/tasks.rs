use clap::Subcommand;
use geekorm::GeekConnector;
use konarr::{models::Projects, utils::grypedb::GrypeDatabase, Config};
use log::{debug, info};

#[derive(Subcommand, Debug, Clone)]
pub enum TaskCommands {
    Grype {
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
        Some(TaskCommands::Grype { alerts }) => {
            info!("Running Grype Sync Task");

            let grype_path = config.data_path()?.join("grypedb");
            debug!("Grype DB Path: {:?}", grype_path);

            GrypeDatabase::sync(&grype_path).await?;

            if alerts {
                info!("Running Grype Alerts Task");
                let grype_conn = GrypeDatabase::connect(&grype_path).await?;

                let mut projects = Projects::fetch_all(&connection).await?;
                info!("Projects Count: {}", projects.len());

                for project in projects.iter_mut() {
                    info!("Project: {}", project.name);
                    if let Some(mut snapshot) = project.fetch_latest_snapshot(&connection).await? {
                        info!("Snapshot: {} :: {}", snapshot.id, snapshot.components.len());

                        let results = snapshot.scan_with_grype(&connection, &grype_conn).await?;
                        info!("Vulnerabilities: {}", results.len());
                    }
                }
            }
        }
        None => {
            info!("No subcommand provided, running interactive mode");
        }
    }

    Ok(())
}
