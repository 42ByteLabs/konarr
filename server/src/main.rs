#![deny(unsafe_code)]

#[macro_use]
extern crate rocket;
extern crate geekorm;

use anyhow::Result;
use konarr::{
    models::{database_initialise, settings::keys::Setting, ServerSettings},
    Config, KonarrError,
};
use log::{debug, error, info, warn};
use rocket::{fs::FileServer, Rocket};
use rocket_cors::{Cors, CorsOptions};
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

mod api;
mod cli;
mod error;
mod guards;
mod routes;

/// Application State
pub struct AppState {
    /// Database Connection
    connection: Arc<Mutex<libsql::Connection>>,
    /// Active sessions for the server
    sessions: Arc<RwLock<Vec<guards::Session>>>,
    /// Token used by the agent to authenticate
    agent_token: Arc<RwLock<String>>,
    /// Configuration
    config: Config,
    /// If the server has been initialized
    init: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let arguments = cli::init();

    let mut config = match Config::load(&arguments.config) {
        Ok(config) => config,
        Err(e) => {
            warn!("Error loading configuration: {}", e);
            let new_config = Config::default();
            warn!("Generating default configuration");
            new_config.autosave()?;
            new_config
        }
    };

    // Database Setup
    let database = config.database().await?;
    let connection = Arc::new(Mutex::new(database.connect()?));

    // konarr::db::init(&connection).await?;
    database_initialise(&mut config, &connection).await?;

    // Tasks
    let task_config = Arc::new(config.clone());
    konarr::tasks::init(task_config, connection).await?;

    // Server
    server(config).await?;

    Ok(())
}

fn cors(config: &Config) -> Result<Cors, KonarrError> {
    if config.server.cors {
        info!("Enabling CORS");
        let cors = if let Some(domain) = config.frontend_url()? {
            info!("CORS Domain: {}", domain);

            CorsOptions {
                // TODO: Update this to be more secure
                allowed_origins: rocket_cors::AllowedOrigins::some_exact(&[domain]),
                allow_credentials: true,
                ..Default::default()
            }
            .to_cors()
            .map_err(|_| KonarrError::UnknownError("Failed to build CORS".to_string()))?
        } else {
            info!("CORS enabled");
            CorsOptions::default()
                .to_cors()
                .map_err(|_| KonarrError::UnknownError("Failed to build CORS".to_string()))?
        };

        Ok(cors)
    } else {
        warn!("CORS is disabled, allowing all origins");
        Ok(CorsOptions {
            allowed_origins: rocket_cors::AllowedOrigins::all(),
            allow_credentials: true,
            ..Default::default()
        }
        .to_cors()
        .map_err(|_| KonarrError::UnknownError("Failed to build CORS".to_string()))?)
    }
}

fn rocket(config: &Config) -> Rocket<rocket::Build> {
    let rocket_config = rocket::Config::figment()
        // Always overwrite the secret key
        .merge(("secret_key", config.server.secret.clone()));

    rocket::custom(rocket_config)
}

async fn server(config: Config) -> Result<()> {
    let frontend = config.frontend_path()?;
    debug!("Frontend Path: {:?}", frontend);
    let cors = cors(&config)?;

    let database = config.database().await?;
    let connection = database.connect()?;
    debug!("Database Initialized");

    // Check if we have init Konarr
    let init: bool = ServerSettings::get_bool(&connection, Setting::Initialized).await?;
    let agent_token: String = ServerSettings::fetch_by_name(&connection, Setting::AgentKey)
        .await?
        .value;

    if !frontend.exists() {
        info!("No Frontend found, creating directory and running in API-only mode");
        std::fs::create_dir_all(&frontend)?;
    }

    let state = AppState {
        connection: Arc::new(Mutex::new(connection)),
        sessions: Arc::new(RwLock::new(Vec::new())),
        agent_token: Arc::new(RwLock::new(agent_token)),
        config: config.clone(),
        init,
    };

    info!("Building Rocket");
    let rocket = rocket(&config)
        .manage(state)
        .attach(cors)
        // Limit
        .register("/", catchers!(guards::limit::rate_limit))
        // Mount Client files
        .mount("/", routes::routes())
        .mount("/", FileServer::from(frontend))
        // Mount API
        .mount("/api", routes![api::base::base])
        .mount("/api/auth", api::auth::routes())
        .mount("/api/projects", api::projects::routes())
        .mount("/api/snapshots", api::snapshots::routes())
        .mount("/api/dependencies", api::dependencies::routes())
        .mount("/api/security", api::security::routes())
        .mount("/api/admin", api::admin::routes())
        .mount("/api", api::websock::routes());

    if let Err(e) = rocket.launch().await {
        error!("Error launching Rocket: {}", e);
        drop(e);
    }

    info!("Stopping Rocket");
    Ok(())
}
