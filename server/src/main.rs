#![doc = include_str!("../../../../README.md")]

#[macro_use]
extern crate rocket;
extern crate geekorm;

use anyhow::Result;
use konarr::{
    models::{self, ServerSettings},
    utils::grypedb::GrypeDatabase,
    Config, KonarrError,
};
use log::{error, info, warn};
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
            new_config.save(&arguments.config)?;
            new_config
        }
    };

    // Database
    create(&mut config).await?;

    info!("Saving Configuration to: {}", arguments.config.display());
    config.save(&arguments.config)?;

    // Server
    server(config).await?;

    Ok(())
}

async fn create(config: &mut Config) -> Result<()> {
    let connection = config.database.connection().await?;

    // TODO: Check if the database exists
    models::database_create(&connection).await?;

    // Store the server setting into the config file
    if let Ok(token) = konarr::models::ServerSettings::fetch_by_name(&connection, "agent.key").await
    {
        config.agent.token = Some(token.value);
    }

    Ok(())
}

async fn advisories(
    config: &Config,
    connection: &libsql::Connection,
) -> Result<Option<libsql::Connection>, KonarrError> {
    let grype_path = config.data_path()?.join("grypedb");
    info!("Grype Path: {:?}", grype_path);

    if ServerSettings::get_bool(connection, "security.advisories.polling").await? {
        info!("Starting Advisory DB Polling");
        match GrypeDatabase::sync(&grype_path).await {
            Ok(_) => {
                info!("Advisory Sync Complete");
            }
            Err(e) => {
                warn!("Advisory Sync Error: {}", e);
            }
        };
        ServerSettings::fetch_by_name(connection, "security.advisories.updated")
            .await?
            .set_update(connection, chrono::Utc::now().to_rfc3339())
            .await?;
    }

    let grype_conn: libsql::Connection = match GrypeDatabase::connect(&grype_path).await {
        Ok(conn) => conn,
        Err(_) => {
            return Ok(None);
        }
    };

    // Set Version
    let grype_id = GrypeDatabase::fetch_grype(&grype_conn).await?;
    ServerSettings::fetch_by_name(connection, "security.advisories.version")
        .await?
        .set_update(connection, grype_id.build_timestamp.to_string().as_str())
        .await?;

    Ok(Some(grype_conn))
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
    let cors = cors(&config)?;

    let database = config.database().await?;
    let connection = database.connect()?;

    // Check if we have init Konarr
    let init: bool = ServerSettings::get_bool(&connection, "initialized").await?;

    // Initialise Security
    if ServerSettings::get_bool(&connection, "security").await? {
        advisories(&config, &connection).await?;
        // Calculate Alerts
        konarr::tasks::alerts::alert_calculator(&connection).await?;
    }

    if !frontend.exists() {
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
