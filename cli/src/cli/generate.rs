use std::path::PathBuf;

use clap::Subcommand;
use konarr::{
    Config,
    bom::{BillOfMaterialsBuilder, cyclonedx::spec_v1_6::Bom as CycloneDx},
    models::{Projects, Snapshot},
};
use log::info;

#[derive(Subcommand, Debug, Clone)]
pub enum GenerateCommands {
    /// Generate a Software Bill of Materials (SBOM)
    Sbom {
        /// Project ID to use for the SBOM
        #[clap(short, long)]
        project: Option<i32>,
        /// Snapshot ID to use for the SBOM
        #[clap(short, long)]
        snapshot: Option<i32>,

        /// Output path for the SBOM file
        #[clap(short, long)]
        output: String,
        /// Path to the SBOM file
        #[clap(short, long, default_value = "cdx")]
        format: String,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<GenerateCommands>,
) -> Result<(), konarr::KonarrError> {
    let database = config.database().await?;

    match subcommands {
        Some(GenerateCommands::Sbom {
            project,
            snapshot,
            output,
            format,
        }) => {
            info!("Generating SBOM in format: {}", format);

            let path = PathBuf::from(output);
            if path.exists() {
                log::info!("Output file already exists, removing: {}", path.display());
                tokio::fs::remove_file(&path).await?;
            }

            let (project, snapshot) = if let Some(project_id) = project {
                let mut project =
                    Projects::fetch_by_primary_key(&database.acquire().await, project_id).await?;

                log::debug!("Fetching latest snapshot");
                let snapshot = project
                    .fetch_latest_snapshot(&database.acquire().await)
                    .await?
                    .cloned();

                (project, snapshot)
            } else if let Some(snap_id) = snapshot {
                let snapshot =
                    Snapshot::fetch_by_primary_key(&database.acquire().await, snap_id).await?;

                log::debug!("Fetching project");
                let project = snapshot.fetch_project(&database.acquire().await).await?;

                (project, Some(snapshot))
            } else {
                return Err(konarr::KonarrError::ParseSBOM(
                    "Project ID or Snapshot ID must be provided".to_string(),
                ));
            };

            let Some(mut snapshot) = snapshot else {
                return Err(konarr::KonarrError::ParseSBOM(
                    "No snapshot found".to_string(),
                ));
            };
            snapshot.fetch_metadata(&database.acquire().await).await?;

            log::info!("Project ID  :: {}", project.id);
            log::info!("Snapshot ID :: {}", snapshot.id);

            let components = snapshot
                .fetch_all_dependencies(&database.acquire().await)
                .await?;
            log::info!("Components  :: {}", components.len());

            match format.as_str() {
                "cdx" | "cyclonedx" => {
                    info!("Generating CycloneDX SBOM");
                    // Build a new SBOM from the components
                    let mut bom = CycloneDx::new();
                    bom.add_project(&project)?;
                    bom.add_dependencies(&components)?;

                    let data = bom.output()?;
                    tokio::fs::write(path, data).await?;
                }
                _ => {
                    info!("Unknown format: {}", format);
                    return Err(konarr::KonarrError::ParseSBOM(
                        "Unknown SBOM format".to_string(),
                    ));
                }
            }
        }
        _ => {
            info!("No subcommand provided");
        }
    }

    Ok(())
}
