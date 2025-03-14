use clap::Subcommand;
use konarr::{Config, models::Dependencies};
use log::{debug, info};

use crate::utils::interactive::prompt_input;

#[derive(Subcommand, Debug, Clone)]
pub enum SearchCommands {
    Name {
        #[clap(short, long)]
        name: String,
    },
    Purl {
        #[clap(short, long)]
        purl: String,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<SearchCommands>,
) -> Result<(), konarr::KonarrError> {
    debug!("Connecting to Database: {:?}", config.database);

    let connection = config.database().await?.connect()?;

    info!("Connected to database!");

    match subcommands {
        Some(SearchCommands::Name { name }) => {
            info!("Searching for Name: {}", name);

            let dependencies = Dependencies::find_by_name(&connection, name).await?;
            display_results(&dependencies);

            Ok(())
        }
        Some(SearchCommands::Purl { purl }) => {
            info!("Searching for PURL: {}", purl);

            let dependencies = Dependencies::find_by_purl(&connection, purl).await?;

            display_results(&dependencies);

            Ok(())
        }
        None => {
            let search = prompt_input("Search for Name or PURL: ")
                .map_err(|e| konarr::KonarrError::UnknownError(e.to_string()))?;

            if search.starts_with("pkg:") {
                let dependencies = Dependencies::find_by_purl(&connection, search).await?;
                display_results(&dependencies);
            } else {
                let dependencies = Dependencies::find_by_name(&connection, search).await?;
                display_results(&dependencies);
            }

            Ok(())
        }
    }
}

fn display_results(dependencies: &Vec<Dependencies>) {
    info!("Instances :: {}", dependencies.len());
    for dep in dependencies.iter() {
        info!(
            " > [{}] {} - {}",
            dep.snapshot_id,
            dep.component_type(),
            dep.purl()
        );
    }
}
