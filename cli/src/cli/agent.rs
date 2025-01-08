use bollard::{container::ListContainersOptions, API_DEFAULT_VERSION};
use konarr::{
    bom::{BomParser, Parsers},
    client::{
        projects::{agent::KonarrProjectSnapshotData, KonarrProject, KonarrProjects},
        snapshot::KonarrSnapshot,
    },
    tools::ToolConfig,
    Config, KonarrError,
};
use log::{debug, info, warn};
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

    let mut tool = if let Some(tool_name) = &config.agent.tool {
        ToolConfig::find_tool(&tool_name).await?
    } else {
        log::error!("Tool not specified");
        return Err(KonarrError::UnknownError("Tool not specified".to_string()));
    };

    info!("Using Tool {}@{}", tool.name, tool.version);

    if !tool.is_available() && config.agent.tool_auto_install {
        info!("Tool not installed, installing...");
        tool.install().await?;
    } else if config.agent.tool_auto_update {
        info!("Checking for tool updates...");
        if let Ok(rversion) = tool.remote_version().await {
            if rversion != tool.version {
                info!("Tool is out of date, updating to {}...", rversion);
                if let Err(err) = tool.install().await {
                    warn!("Failed to update tool: {}", err);
                }
            }
        } else {
            warn!("Failed to get remote version of tool, skipping update");
        }
    }

    debug!("Getting Docker Version and updating Snapshot Metadata");
    let version = docker.version().await?;
    let engine = version.platform.unwrap_or_default().name;

    let mut server_snapshot = server_project.snapshot.clone().expect(
        "Snapshot is required to update metadata. Please create a snapshot before running this command");

    server_snapshot.add_metadata("os".to_string(), version.os.unwrap_or_default());
    server_snapshot.add_metadata(
        "os.kernel".to_string(),
        version.kernel_version.unwrap_or_default(),
    );
    server_snapshot.add_metadata("os.arch".to_string(), version.arch.unwrap_or_default());
    server_snapshot.add_metadata("container".to_string(), "true".to_string());
    server_snapshot.add_metadata("container.engine".to_string(), engine);
    server_snapshot.add_metadata(
        "container.engine.version".to_string(),
        version.version.unwrap_or_default(),
    );

    info!("Updated server snapshot metadata...");
    server_snapshot.update_metadata(client).await?;

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

        let container_image = container.image.clone().unwrap_or_default();

        let snapshot_data = KonarrProjectSnapshotData {
            container_sha: container.image_id.clone(),
            tool: Some(format!("{}@{}", tool.name, tool.version)),
        };

        let mut container_snapshot = project.snapshot(client, &snapshot_data).await?;
        debug!("Container Snapshot: {}", container_snapshot.id);

        if container_snapshot.new {
            info!("Running tool on container...");
            let results = tool.run(container_image).await?;

            log::info!("Parsing and validating SBOM with Konarr...");
            match Parsers::parse(&results.as_bytes()) {
                Ok(bom) => {
                    info!("Validate SBOM spec supported by Konarr: {}", bom.sbom_type);
                }
                Err(e) => {
                    return Err(KonarrError::UnknownError(
                        format!("Error parsing SBOM: {:?}", e).to_string(),
                    ));
                }
            }

            info!("Uploading BOM to Server...");
            let json_data: serde_json::Value = serde_json::from_slice(&results.as_bytes())?;

            let result = container_snapshot.upload_bom(client, json_data).await?;
            info!("Uploaded BOM to Server");
            debug!("Snapshot: {:#?}", result);
        } else {
            info!("Container Snapshot already exists for Container: {}", name);
        }

        let image_name = container.image.clone().unwrap_or_default();

        container_snapshot.add_metadata("container", "true");

        if let Ok(image) = docker.inspect_image(&image_name).await {
            // https://docs.rs/bollard/latest/bollard/models/struct.ImageInspect.html
            debug!("Image: {:#?}", image);
            container_snapshot.add_metadata("container.image", &image_name);
            container_snapshot
                .add_metadata("container.image.created", image.created.unwrap_or_default());
            container_snapshot.add_metadata("container.image.os", image.os.unwrap_or_default());
            container_snapshot.add_metadata(
                "container.image.arch",
                image.architecture.unwrap_or_default(),
            );
            container_snapshot.add_metadata(
                "container.image.variant",
                image.os_version.unwrap_or_default(),
            );
        }

        // We always update the metadata for the container snapshot
        // https://docs.rs/bollard/latest/bollard/models/struct.ContainerSummary.html
        container_snapshot.add_metadata(
            "container.sha",
            container.image_id.clone().unwrap_or_default(),
        );
        container_snapshot.add_metadata(
            // Container Created
            "container.created",
            chrono::DateTime::from_timestamp_nanos(container.created.unwrap_or_default())
                .to_string(),
        );

        if let Some(url) = labels.get("org.opencontainers.image.url") {
            container_snapshot.add_metadata("container.image.url", url);
        }
        if let Some(licenses) = labels.get("org.opencontainers.image.licenses") {
            container_snapshot.add_metadata("container.image.licenses", licenses);
        }
        if let Some(version) = labels.get("org.opencontainers.image.version") {
            container_snapshot.add_metadata("container.image.version", version);
        }
        if let Some(authors) = labels.get("org.opencontainers.image.authors") {
            container_snapshot.add_metadata("container.image.authors", authors);
        }

        // History
        if let Ok(history) = docker.image_history(&image_name).await {
            let history_items = history
                .iter()
                .rev()
                .map(|h| h.created_by.clone())
                .collect::<Vec<_>>();

            debug!("History: {:#?}", history_items);
            container_snapshot.add_metadata(
                "container.image.history",
                serde_json::to_string(&history_items)?,
            );
        }

        container_snapshot.update_metadata(client).await?;

        info!("Done with Container: {}", name);
    }

    Ok(())
}
