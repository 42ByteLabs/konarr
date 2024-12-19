use bollard::{container::ListContainersOptions, API_DEFAULT_VERSION};
use konarr::{
    client::{
        projects::{KonarrProject, KonarrProjects},
        snapshot::KonarrSnapshot,
    },
    Config, KonarrError,
};
use log::{debug, info};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Mutex};
use tokio_schedule::{every, Job};

pub async fn setup(
    config: &Config,
    client: &konarr::client::KonarrClient,
) -> Result<(), konarr::KonarrError> {
    // ID -> Hostname -> New Project
    let mut project = if let Some(project_id) = config.agent.project_id {
        log::debug!("Project ID :: {}", project_id);

        match KonarrProjects::by_id(&client, project_id).await {
            Ok(Some(project)) => project,
            _ => {
                log::error!("Failed to get project by id: {}", project_id);
                return Err(KonarrError::KonarrClient(
                    "Failed to get project by id".to_string(),
                ));
            }
        }
    } else if let Some(hostname) = &config.agent.host {
        log::debug!("Hostname :: {}", hostname);
        let lhost = hostname.to_lowercase();

        // Look at top projects
        let project: Option<KonarrProject> = KonarrProjects::by_name(&client, &lhost).await?;

        match project {
            Some(p) => p,
            None => {
                log::debug!("Project not found by name: {}", hostname);
                if !config.agent.create {
                    log::error!("Failed to get project by name: {}", hostname);
                    return Err(KonarrError::KonarrClient(
                        "Failed to get project by name".to_string(),
                    ));
                }
                // Auto-Create Projects
                log::info!("Auto-Create mode enabled");
                KonarrProject::new(hostname, "Server")
                    .create(&client)
                    .await?
            }
        }
    } else {
        // TODO: Auto-Create Projects
        log::error!("Failed to get project by ID or Name");
        return Err(KonarrError::UnknownError(
            "Unknown project id / name".to_string(),
        ));
    };

    info!("Project :: {}", project.id);
    debug!("Project Snapshot :: {:?}", project.snapshot);

    // TODO: Multi-threading is hard...
    let config = Arc::new(config.clone());
    let client = Arc::new(client.clone());

    log::info!("Running agent!");
    run(&config, &client, &mut project).await?;

    if config.agent.monitoring {
        info!("Monitoring mode enabled");

        let task = every(1).minutes().perform(move || {
            // Only allow one task to run at a time, skip if already running
            let active = Mutex::new(false);

            // TODO: Multi-threading is hard...
            let config = config.clone();
            let client = client.clone();
            let mut project = project.clone();

            async move {
                info!("Running monitoring task...");

                if *active.lock().await {
                    info!("Task already running... Skipping");
                    return;
                }
                run(&config, &client, &mut project)
                    .await
                    .expect("Panic in monitoring mode...");

                info!("Finishing task... Waiting for next");
            }
        });
        spawn(task).await.expect("Panic in monitoring mode...");
    }
    Ok(())
}

async fn run(
    config: &Config,
    client: &konarr::client::KonarrClient,
    project: &mut KonarrProject,
) -> Result<(), konarr::KonarrError> {
    // The host
    debug!("Host Project :: {:?}", project);
    let snapshot = if let Some(snap) = project.snapshot.clone() {
        snap
    } else {
        info!("Creating Host Snapshot...");
        KonarrSnapshot::create(client, project.id).await?
    };

    debug!("Snapshot: {:#?}", snapshot);
    project.snapshot = Some(snapshot);

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

    Ok(())
}

async fn run_docker(
    config: &Config,
    socket: Option<String>,
    client: &konarr::client::KonarrClient,
    server_project: &KonarrProject,
) -> Result<(), konarr::KonarrError> {
    let docker = if let Some(socket) = socket {
        bollard::Docker::connect_with_local(&socket, 120, API_DEFAULT_VERSION)?
    } else {
        bollard::Docker::connect_with_unix_defaults()?
    };
    info!("Connected to Docker");

    debug!("Getting Docker Version and updating Snapshot Metadata");
    let version = docker.version().await?;
    let engine = version.platform.unwrap_or_default().name;

    let server_snapshot = server_project.snapshot.clone().expect(
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
    info!("Updated server snapshot metadata...");

    info!("Getting Docker Containers...");
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            filters: HashMap::from([("status".to_string(), vec!["running".to_string()])]),
            ..Default::default()
        }))
        .await?;

    let prefix = server_project.name.clone();

    for container in containers {
        let labels = container.labels.clone().unwrap_or_default();

        let name: String = if let Some(project) = labels.get("com.docker.compose.project") {
            // From Compose metadata (`project` is folder, `service` is name)
            if let Some(service) = labels.get("com.docker.compose.service") {
                format!("{}/{}/{}", prefix, project, service)
            } else {
                format!("{}/{}", prefix, project)
            }
        } else if let Some(title) = labels.get("org.opencontainers.image.title") {
            // Name of the container
            format!("{}/{}", prefix, title.clone())
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
                    proj.parent = Some(server_project.id);
                    proj.description = description.clone();
                    proj.create(client).await?;
                    proj
                }
            }
        } else {
            info!("Creating new Project for Container: {}", name);
            let mut proj = KonarrProject::new(name.clone(), "container".to_string());
            proj.parent = Some(server_project.id);
            proj.description = description.clone();
            proj.create(client).await?;
            proj
        };

        project.get(client).await?;
        info!("Project: {} - {}", project.name, project.project_type);

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
                    match KonarrSnapshot::create(client, project.id).await {
                        Ok(snap) => (true, snap),
                        Err(e) => {
                            log::error!("Error creating Snapshot: {:?}", e);
                            (false, snap)
                        }
                    }
                }
            } else {
                debug!("Creating new Snapshot for Container: {}", name);
                match KonarrSnapshot::create(client, project.id).await {
                    Ok(snap) => (true, snap),
                    Err(e) => {
                        log::error!("Error creating Snapshot: {:?}", e);
                        (false, snap)
                    }
                }
            }
        } else {
            info!("Creating initial Snapshot...");
            match KonarrSnapshot::create(client, project.id).await {
                Ok(snap) => (true, snap),
                Err(e) => {
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
            let results = konarr::tools::run(&config, container_image).await?;

            info!("Uploading BOM to Server");
            container_snapshot.upload_bom(client, results).await?;
        } else {
            info!("Container Snapshot already exists for Container: {}", name);
        }

        info!("Done with Container: {}", name);
    }

    Ok(())
}
