//! # Task - Advisories

use crate::{models::ServerSettings, utils::grypedb::GrypeDatabase, Config, KonarrError};
use geekorm::prelude::*;
use log::{info, warn};

/// Poll for Advisories and update the database
pub async fn sync_advisories<'a, T>(
    config: &'a Config,
    connection: &'a T,
) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    if !ServerSettings::get_bool(connection, "security.advisories").await? {
        info!("Advisories Disabled");
        return Ok(());
    }

    let grype_path = config.grype_path()?;
    info!("Grype Path: {:?}", grype_path);

    if ServerSettings::get_bool(connection, "security.advisories.polling").await? {
        info!("Starting Advisory DB Polling");
        match GrypeDatabase::sync(&grype_path).await {
            Ok(_) => {
                info!("Advisory Sync Complete");
            }
            Err(e) => {
                warn!("Advisory Sync Error: {}", e);
                reset_polling(connection).await?;
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
            return Ok(());
        }
    };

    // Set Version
    let grype_id = match GrypeDatabase::fetch_grype(&grype_conn).await {
        Ok(grype) => grype,
        Err(_) => {
            warn!("Errors loading Grype DB");
            reset_polling(connection).await?;

            return Ok(());
        }
    };
    ServerSettings::fetch_by_name(connection, "security.advisories.version")
        .await?
        .set_update(connection, grype_id.build_timestamp.to_string().as_str())
        .await?;

    Ok(())
}

async fn reset_polling<'a, T>(connection: &'a T) -> Result<(), KonarrError>
where
    T: GeekConnection<Connection = T> + Send + Sync + 'a,
{
    ServerSettings::fetch_by_name(connection, "security.advisories.polling")
        .await?
        .set_update(connection, "disabled")
        .await?;

    Ok(())
}
