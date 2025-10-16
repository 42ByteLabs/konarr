use clap::Subcommand;
use geekorm::prelude::*;
use konarr::{
    Config,
    bom::BomParser,
    models::{ProjectType, Projects, Snapshot},
};
use log::{debug, info};
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum IndexCommand {
    Sbom {
        #[clap(long)]
        path: PathBuf,
        #[clap(long)]
        format: Option<String>,
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

    let database = config.database().await?;
    let connection = database.acquire().await;

    info!("Connected to database!");

    match subcommands {
        Some(IndexCommand::Sbom { path, format }) => {
            info!("Running SBOM Command");

            if !path.exists() {
                return Err(konarr::KonarrError::UnknownError(
                    path.display().to_string(),
                ));
            }

            if path.is_file() {
                // Find or Create the project
                let mut project = if let Some(project_id) = config.agent.project_id {
                    Projects::fetch_by_primary_key(&connection, project_id as i32).await?
                } else if let Some(project_name) = &config.agent.host {
                    Projects::fetch_by_name(&connection, project_name).await?
                } else {
                    let input = crate::utils::interactive::prompt_input("Project Name")
                        .expect("Failed to get input");
                    let mut proj = Projects::new(input, ProjectType::Container);
                    proj.fetch_or_create(&connection).await?;
                    proj
                };
                info!("Project Name :: {:?}", project);

                info!("File Path: {:?}", path);
                let data = tokio::fs::read(&path).await?;

                let bom = if let Some(frmt) = format {
                    konarr::bom::Parsers::parse_with_name(&data, frmt)?
                } else {
                    konarr::bom::Parsers::parse(&data)?
                };

                info!("BOM Type            :: {}", bom.sbom_type);
                info!("BOM Version         :: {}", bom.version);
                info!("BOM SHA             :: {}", bom.sha);
                for (index, tool) in bom.tools.iter().enumerate() {
                    info!(
                        "BOM Tool [{}]        :: {} ({})",
                        index, tool.name, tool.version
                    );
                }
                info!("BOM Dependencies    :: {}", bom.components.len());
                info!("BOM Vulnerabilities :: {}", bom.vulnerabilities.len());

                let mut snapshot = Snapshot::from_bom(&connection, &bom).await?;
                info!("Snapshot ID: {:?}", snapshot.id);

                snapshot.add_bom(&connection, data).await?;
                snapshot
                    .set_state(&connection, konarr::models::SnapshotState::Completed)
                    .await?;

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
