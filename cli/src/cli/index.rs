use clap::Subcommand;
use geekorm::prelude::*;
use konarr::{
    bom::BomParser,
    models::{ProjectType, Projects, Snapshot},
    Config,
};
use log::{debug, info};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum IndexCommand {
    Sbom {
        #[clap(short, long)]
        project_name: String,

        #[clap(long)]
        path: PathBuf,
    },
    Advisories {
        #[clap(short, long)]
        path: PathBuf,

        #[clap(long)]
        source: String,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<IndexCommand>,
) -> Result<(), konarr::KonarrError> {
    debug!("Connecting to Database: {:?}", config.database);

    let connection = config.database().await?.connect()?;

    info!("Connected to database!");

    match subcommands {
        Some(IndexCommand::Sbom { project_name, path }) => {
            info!("Running SBOM Command");

            if !path.exists() {
                return Err(konarr::KonarrError::UnknownError(
                    path.display().to_string(),
                ));
            }

            if path.is_file() {
                info!("Project Name :: {:?}", project_name);

                // Find or Create the project
                let mut project = Projects::new(project_name, ProjectType::Container);
                project.fetch_or_create(&connection).await?;

                info!("File Path: {:?}", path);
                let bom = konarr::bom::Parsers::parse_path(path)?;

                info!("BOM Type            :: {}", bom.sbom_type);
                info!("BOM Version         :: {}", bom.version);
                info!("BOM SHA             :: {}", bom.sha);
                for (index, tool) in bom.tools.iter().enumerate() {
                    info!(
                        "BOM Tool [{}]         :: {} ({})",
                        index, tool.name, tool.version
                    );
                }
                info!("BOM Dependencies    :: {}", bom.components.len());

                let snapshot = Snapshot::from_bom(&connection, &bom).await?;

                info!("Snapshot ID: {:?}", snapshot.id);

                project.add_snapshot(&connection, snapshot).await?;
            } else if path.is_dir() {
                todo!("Directory Parsing");
            }

            Ok(())
        }
        _ => {
            println!("No subcommand provided");
            Ok(())
        }
    }
}
