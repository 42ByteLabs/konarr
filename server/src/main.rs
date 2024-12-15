#![deny(unsafe_code)]

#[macro_use]
extern crate rocket;
extern crate geekorm;

use std::sync::Arc;

use anyhow::Result;
use konarr::{
    models::{database_create, settings::keys::Setting, ServerSettings},
    Config, KonarrError,
};
use log::{debug, error, info, warn};
use rocket::{fs::FileServer, Rocket};
use rocket_cors::{Cors, CorsOptions};

mod api;
mod cli;
mod error;
mod guards;
mod routes;

/// Application State
pub struct AppState {
    db: libsql::Database,
    config: Config,
    init: bool,
}

#[rocket::main]
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

    // Database
    create(&mut config).await?;

    // Tasks
    let task_config = Arc::new(config.clone());
    let database = Arc::new(config.database().await?);
    konarr::tasks::init(task_config, database).await?;

    // Server
    server(config).await?;

    Ok(())
}

/// Setup, Create, and Update the Database
///
/// - Run Create Database
/// - Initiale data
/// - Update Statistics
/// - Update Security Data
async fn create(config: &mut Config) -> Result<()> {
    let connection = config.database.connection().await?;

    // TODO: Check if the database exists
    database_create(&connection).await?;

    // Store the server setting into the config file
    if config.agent.token.is_none() {
        if let Ok(token) =
            konarr::models::ServerSettings::fetch_by_name(&connection, "agent.key").await
        {
            config.agent.token = Some(token.value);
            config.autosave()?;
        }
    }

    // Update Stats
    konarr::tasks::statistics(&connection).await?;

    // Initialise Security
    if ServerSettings::get_bool(&connection, Setting::Security).await? {
        if ServerSettings::get_bool(&connection, Setting::SecurityAdvisories).await? {
            debug!("Syncing Security Advisories");

            match konarr::tasks::advisories::sync_advisories(&config, &connection).await {
                Err(e) => {
                    error!("{}", e);
                    ServerSettings::fetch_by_name(&connection, Setting::SecurityAdvisories)
                        .await?
                        .set_update(&connection, "disabled")
                        .await?;
                }
                _ => {}
            }
        } else {
            debug!("Security Advisories are disabled");
        }
        // Calculate Alerts
        konarr::tasks::alerts::alert_calculator(&connection).await?;
    }

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

    if !frontend.exists() {
        info!("No Frontend found, creating directory and running in API-only mode");
        std::fs::create_dir_all(&frontend)?;
    }

    let state = AppState {
        db: database,
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
        .register("/", catchers![routes::failed_not_found])
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
