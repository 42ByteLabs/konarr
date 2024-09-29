use std::{collections::HashMap, path::PathBuf};

use bollard::{container::ListContainersOptions, API_DEFAULT_VERSION};
use clap::Subcommand;
use konarr::{
    client::{projects::KonarrProject, snapshot::KonarrSnapshot, ApiResponse},
    tools::{syft::Syft, Tool},
    Config, KonarrError,
};
use log::{debug, info};

#[derive(Subcommand, Debug, Clone)]
pub enum AgentCommands {
    Docker {
        #[clap(short, long, env = "DOCKER_HOST")]
        socket: Option<String>,

        #[clap(long, env = "HOST")]
        host: Option<String>,
    },
}

pub async fn run(
    config: &Config,
    subcommands: Option<AgentCommands>,
    client: &konarr::client::KonarrClient,
    project: KonarrProject,
) -> Result<(), konarr::KonarrError> {
    let mut project = project.clone();
    let snapshot = if let Some(snap) = project.snapshot {
        snap
    } else {
        info!("Creating Snapshot...");
        match KonarrSnapshot::create(client, project.id).await? {
            ApiResponse::Ok(snap) => snap,
            ApiResponse::Error(e) => {
                log::error!("Error creating Snapshot: {:?}", e);
                return Err(KonarrError::UnknownError("Snapshot".to_string()));
            }
        }
    };
    debug!("Snapshot: {:#?}", snapshot);
    project.snapshot = Some(snapshot);

    match subcommands {
        #[allow(unused_variables)]
        Some(AgentCommands::Docker { socket, host }) => {
            run_docker(config, socket, client, project).await?;
            Ok(())
        }
        None => {
            info!("Auto-Discover mode...");

            // Docker
            match std::env::var("DOCKER_HOST") {
                Ok(socket) => {
                    info!("Using Docker Socket: {}", socket);
                    run_docker(config, Some(socket), client, project).await?;
                }
                Err(_) => {
                    let docker_socket = PathBuf::from("/var/run/docker.sock");
                    if docker_socket.exists() {
                        info!("Using Docker Socket: {:?}", docker_socket);
                        run_docker(
                            config,
                            Some(docker_socket.to_str().unwrap().to_string()),
                            client,
                            project,
                        )
                        .await?;
                    }
                }
            }

            Err(KonarrError::UnknownError("No Agent found".to_string()))
        }
    }
}

pub async fn run_docker(
    _config: &Config,
    socket: Option<String>,
    client: &konarr::client::KonarrClient,
    server_project: KonarrProject,
) -> Result<(), konarr::KonarrError> {
    info!("Docker Monitor Command");

    let docker = if let Some(socket) = socket {
        bollard::Docker::connect_with_local(&socket, 120, API_DEFAULT_VERSION)?
    } else {
        bollard::Docker::connect_with_unix_defaults()?
    };
    info!("Connected to Docker");

    info!("Getting Docker Version and updating Snapshot Metadata");
    let version = docker.version().await?;
    let engine = version.platform.unwrap_or_default().name;

    let server_snapshot = server_project.snapshot.expect(
        "Snapshot is required to update metadata. Please create a snapshot before running this command");

    server_snapshot
        .update_metadata(
            client,
            HashMap::from([
                ("os", version.os.unwrap_or_default()),
                ("os.kernel", version.kernel_version.unwrap_or_default()),
                ("os.arch", version.arch.unwrap_or_default()),
                ("container", "true".to_string()),
                ("container.engine", engine),
                (
                    "container.engine.version",
                    version.version.unwrap_or_default(),
                ),
            ]),
        )
        .await?;

    info!("Getting Docker Containers...");
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            filters: HashMap::from([("status".to_string(), vec!["running".to_string()])]),
            ..Default::default()
        }))
        .await?;

    for container in containers {
        let labels = container.labels.clone().unwrap_or_default();

        let name: String = if let Some(title) = labels.get("org.opencontainers.image.title") {
            // Name of the container
            title.clone()
        } else if let Some(compose_project) = labels.get("com.docker.compose.project") {
            // From Compose metadata
            compose_project.clone()
        } else if let Some(names) = &container.names {
            names.first().unwrap().replacen("/", "", 1)
        } else if let Some(image) = &container.image {
            image.to_string()
        } else {
            return Err(KonarrError::UnknownError("Container Name".to_string()));
        };

        info!("Container: {:?}", name);

        let description: Option<String> =
            labels.get("org.opencontainers.image.description").cloned();

        let mut project: KonarrProject = if let Some(children) = &server_project.children {
            match children.iter().find(|p| p.name == name) {
                Some(project) => {
                    info!("Found Project for Container: {}", project.name);
                    project.clone()
                }
                None => {
                    info!("Creating new Project for Container: {}", name);
                    let mut proj = KonarrProject::new(name.clone(), "container".to_string());
                    proj.parent = Some(server_project.id as i32);
                    proj.description = description.clone();
                    proj.create(client).await?;
                    proj
                }
            }
        } else {
            info!("Creating new Project for Container: {}", name);
            let mut proj = KonarrProject::new(name.clone(), "container".to_string());
            proj.parent = Some(server_project.id as i32);
            proj.description = description.clone();
            proj.create(client).await?;
            proj
        };

        project.get(client).await?;
        info!("Project: {} - {}", project.name, project.r#type);

        let container_sha = container.image_id.clone().unwrap_or_default();
        let container_image = container.image.clone().unwrap_or_default();

        // The SHA is used to identify the container snapshot
        // and check if the snapshot already exists
        let (state, container_snapshot) = if let Some(snap) = project.snapshot {
            if let Some(sha) = snap.metadata.get("container.sha") {
                debug!("Container Snapshot SHA: {} == {}", &container_sha, sha);
                if sha == &container_sha {
                    debug!("Container Snapshot already exists for Container: {}", name);
                    (false, snap)
                } else {
                    debug!("Snapshot SHA for Container is different: {}", name);
                    match KonarrSnapshot::create(client, project.id).await? {
                        ApiResponse::Ok(snap) => (true, snap),
                        ApiResponse::Error(e) => {
                            log::error!("Error creating Snapshot: {:?}", e);
                            (false, snap)
                        }
                    }
                }
            } else {
                debug!("Creating new Snapshot for Container: {}", name);
                match KonarrSnapshot::create(client, project.id).await? {
                    ApiResponse::Ok(snap) => (true, snap),
                    ApiResponse::Error(e) => {
                        log::error!("Error creating Snapshot: {:?}", e);
                        (false, snap)
                    }
                }
            }
        } else {
            info!("Creating initial Snapshot...");
            match KonarrSnapshot::create(client, project.id).await? {
                ApiResponse::Ok(snap) => (true, snap),
                ApiResponse::Error(e) => {
                    log::error!("Error creating Snapshot: {:?}", e);
                    return Err(KonarrError::UnknownError(
                        "Error creating initial Snapshot".to_string(),
                    ));
                }
            }
        };

        info!("Container Snapshot: {}", container_snapshot.id);

        // TODO: Docker Compose metadata
        // TODO: Creation time of the container

        // We always update the metadata for the container snapshot
        let snapshot_metadata = HashMap::from([
            ("container", "true".to_string()),
            ("container.image", container.image.unwrap_or_default()),
            ("container.sha", container_sha),
            ("container.description", description.unwrap_or_default()),
            (
                "container.url",
                labels
                    .get("org.opencontainers.image.url")
                    .cloned()
                    .unwrap_or_default(),
            ),
            (
                "container.licenses",
                labels
                    .get("org.opencontainers.image.licenses")
                    .cloned()
                    .unwrap_or_default(),
            ),
            (
                "container.version",
                labels
                    .get("org.opencontainers.image.version")
                    .cloned()
                    .unwrap_or_default(),
            ),
            (
                "container.authors",
                labels
                    .get("org.opencontainers.image.authors")
                    .cloned()
                    .unwrap_or_default(),
            ),
        ]);
        container_snapshot
            .update_metadata(client, snapshot_metadata)
            .await?;

        if state {
            let tool = Syft::init().await?;
            info!("Running Syft on Container: {}", name);

            let results = tool.run(container_image).await?;
            debug!("Syft Results: {:#?}", results);

            info!("Uploading BOM to Server");
            container_snapshot.upload_bom(client, results).await?;
        } else {
            info!("Container Snapshot already exists for Container: {}", name);
        }

        info!("Done with Container: {}", name);
    }

    Ok(())
}
