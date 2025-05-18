//! # Server Settings from Config
use super::{ServerSettings, keys::Setting};
use crate::Config;
use geekorm::{Connection, prelude::*};

impl ServerSettings {
    /// The server configuration is the source of truth as the
    /// user can override the settings in the config file but we
    /// need to make sure that the settings are in the database.
    ///
    /// When it gets to this point, the database settings should
    /// already be added so this is just a matter of updating the
    /// settings.
    pub async fn load_config(
        connection: &Connection<'_>,
        config: &Config,
    ) -> Result<(), crate::KonarrError> {
        log::debug!("Loading server settings from config");

        // Server settings
        if let Ok(url) = config.server.url() {
            log::debug!("Server URL: {}", url);
            ServerSettings::update_setting(connection, &Setting::ServerUrl, url.to_string())
                .await?;
        }
        // Data path
        ServerSettings::update_setting(
            connection,
            &Setting::ServerData,
            config.data_path()?.canonicalize()?.display().to_string(),
        )
        .await?;

        // Frontend setting
        ServerSettings::update_setting(
            connection,
            &Setting::ServerFrontendPath,
            config.server.frontend.canonicalize()?.display().to_string(),
        )
        .await?;

        Ok(())
    }

    async fn update_setting(
        connection: &Connection<'_>,
        setting: &Setting,
        value: impl Into<String>,
    ) -> Result<(), crate::KonarrError> {
        let value = value.into();
        log::debug!("Updating setting: {:?}", setting);

        let mut dbsetting = ServerSettings::fetch_by_name(connection, setting).await?;

        if dbsetting.value != value {
            log::debug!("Updating setting: {:?}", setting);
            dbsetting.value = value;
            dbsetting.update(connection).await?;
        } else {
            log::debug!("Setting already up to date: {:?}", setting);
        }

        Ok(())
    }
}
