//! # Server Settings from Config
use super::SettingType;
use super::{ServerSettings, keys::Setting};
use crate::Config;
use geekorm::{Connection, prelude::*};

impl ServerSettings {
    /// The server configuration is the source of truth as the
    /// user can override the settings in the config file but we
    /// need to make sure that the settings are in the database.
    pub async fn load_config(
        connection: &Connection<'_>,
        config: &Config,
    ) -> Result<(), crate::KonarrError> {
        // Server settings
        if let Ok(url) = config.server.url() {
            log::debug!("Server URL: {}", url);
            ServerSettings::update_setting(connection, &Setting::ServerUrl, url.to_string())
                .await?;
        }

        // Frontend setting
        ServerSettings::update_setting(
            connection,
            &Setting::ServerFrontendPath,
            config.server.frontend.display().to_string(),
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

        let mut dbsetting = match ServerSettings::fetch_by_name(connection, setting).await {
            Ok(setting) => setting,
            Err(_) => {
                log::debug!("Creating new setting: {:?}", setting);
                let mut sett =
                    ServerSettings::new(setting.clone(), SettingType::String, value.clone());
                sett.fetch_or_create(connection).await?;
                sett
            }
        };

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
