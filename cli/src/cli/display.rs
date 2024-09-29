use clap::Subcommand;
use console::style;
use konarr::{
    models::{Dependencies, Snapshot},
    Config,
};
use log::{debug, info};

#[derive(Subcommand, Debug, Clone)]
pub enum DisplayCommands {
    Snapshots {
        #[clap(short, long)]
        id: Option<i32>,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<DisplayCommands>,
) -> Result<(), konarr::KonarrError> {
    debug!("Connecting to Database: {:?}", config.database);

    let connection = config.database().await?.connect()?;

    info!("Connected to database!");

    match subcommands {
        Some(DisplayCommands::Snapshots { id }) => {
            info!("Displaying Snapshots");

            if let Some(id) = id {
                let mut snapshot = Snapshot::fetch_by_primary_key(&connection, id).await?;
                snapshot.fetch_metadata(&connection).await?;

                println!("Snapshot ID: {:?}", snapshot.id);
                // Display Snapshot Details
                for (name, md) in snapshot.metadata.iter() {
                    println!(" > {}: {}", name, md.as_string());
                }

                let dependencies =
                    Dependencies::fetch_dependencies_by_snapshop(&connection, snapshot.id).await?;

                println!("Dependencies :: {}", dependencies.len());
                for dep in dependencies.iter() {
                    println!(" > [{}] {}", dep.component_type(), dep.purl());
                }

                Ok(())
            } else {
                let mut snapshots = Snapshot::all(&connection).await?;

                for snap in snapshots.iter_mut() {
                    println!("Snapshot ID: {:?}", style(snap.id).red());

                    snap.fetch_metadata(&connection).await?;

                    for (name, meta) in snap.metadata.iter() {
                        let md_value = meta.as_string();
                        println!(
                            " > {}: {}",
                            style(name.clone()).blue(),
                            style(md_value).green()
                        );
                    }
                }

                // Summary of all snaps
                Ok(())
            }
        }
        None => {
            println!("No subcommand provided");
            Ok(())
        }
    }
}
