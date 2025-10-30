//! # Server Settings Model
use geekorm::{Connection, prelude::*};
use keys::SERVER_SETTINGS_DEPRICATED;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};

pub mod config;
pub mod defaults;
pub mod keys;

pub use defaults::SERVER_SETTINGS_DEFAULTS;
pub use keys::Setting;

/// Setting Type
#[derive(Data, Debug, Default, Clone, PartialEq)]
pub enum SettingType {
    /// Toggle (enabled/disabled)
    Toggle,
    /// Regenerate value (e.g. API Key)
    Regenerate,
    /// User controllable setting
    SetString,

    /// Boolean
    Boolean,
    /// String
    #[default]
    String,
    /// Integer
    Integer,
    /// Float
    Float,

    /// Datetime (UTC)
    Datetime,

    /// Statistics (unsigned integer)
    ///
    /// This is used for counters, etc. and should not be used for
    /// settings that require a specific value.
    Statistics,

    /// Delete (this is for cleanup purposes)
    Delete,
}

/// Server Settings Table
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Setting Name
    #[geekorm(unique, not_null)]
    pub name: Setting,

    /// Setting Type
    pub setting_type: SettingType,

    /// Setting Value
    #[geekorm(not_null)]
    pub value: String,

    /// Updated At Datetime
    #[geekorm(new = "chrono::Utc::now()", on_update = "chrono::Utc::now()")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Find setting in the list
pub fn find_setting(settings: &[ServerSettings], name: Setting) -> Option<&ServerSettings> {
    settings.iter().find(|s| s.name == name)
}
/// Find setting in the list and default if not present
pub fn find_statistic(settings: &[ServerSettings], name: Setting) -> u64 {
    settings
        .iter()
        .find(|s| s.name == name)
        .map_or(0, |s| s.value.parse().unwrap_or(0))
}

impl ServerSettings {
    /// Initialize the Server Settings Table
    pub async fn init(connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        for (name, typ, value) in Self::defaults() {
            match ServerSettings::fetch_by_name(connection, name.to_string()).await {
                Ok(mut setting) => {
                    // Update setting type in case it has changed in newer versions
                    if setting.setting_type != typ {
                        debug!("Updating setting: {:?}", name);
                        setting.setting_type = typ;
                        setting.update(connection).await?;
                    }
                }
                Err(geekorm::Error::SerdeError(e)) => {
                    error!("Error fetching setting: `{}` ({})", name, e);
                    return Err(crate::KonarrError::GeekOrm(geekorm::Error::SerdeError(e)));
                }
                Err(e) => {
                    debug!("Creating setting: `{}` ({})", name, e);
                    let mut setting = ServerSettings::new(name, typ, value);
                    setting.save(connection).await?;
                }
            };
        }

        // Deprecate old settings
        for depricated in SERVER_SETTINGS_DEPRICATED {
            if let Ok(setting) = ServerSettings::fetch_by_name(connection, &depricated).await {
                warn!("Deprecating setting: {:?}", depricated);
                setting.delete(connection).await?;
            }
        }

        Ok(())
    }

    /// Create a default list of ServerSettings entries
    fn defaults() -> Vec<(Setting, SettingType, String)> {
        let mut defaults: Vec<(Setting, SettingType, String)> = SERVER_SETTINGS_DEFAULTS
            .to_vec()
            .into_iter()
            .map(|(a, b, c)| (a, b, c.to_string()))
            .collect();

        let agent_key = geekorm::utils::generate_random_string(43, "kagent_");
        defaults.push((Setting::AgentKey, SettingType::Regenerate, agent_key));

        defaults
    }

    /// Fetch all the settings that are not statistics
    pub async fn fetch_settings(
        connection: &Connection<'_>,
    ) -> Result<Vec<ServerSettings>, crate::KonarrError> {
        Ok(ServerSettings::query(
            connection,
            ServerSettings::query_select()
                .where_ne("setting_type", SettingType::Statistics)
                .build()?,
        )
        .await?)
    }

    /// Update Statistic Setting
    pub async fn update_statistic(
        connection: &Connection<'_>,
        name: Setting,
        value: i64,
    ) -> Result<(), crate::KonarrError> {
        match ServerSettings::fetch_by_name(connection, &name).await {
            Ok(mut setting) => {
                if value != setting.value.parse().unwrap_or(0) {
                    debug!(
                        "Updating statistic: {:?} = {} (was {})",
                        name, value, setting.value
                    );
                    setting.value = value.to_string();
                    setting.update(connection).await?;
                }
            }
            Err(_) => {
                let mut setting =
                    ServerSettings::new(name, SettingType::Statistics, value.to_string());
                setting.save(connection).await?;
            }
        }
        Ok(())
    }

    /// Set the Setting
    pub fn set(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.setting_type == SettingType::Boolean {
            debug!("Setting boolean: {:?} = {}", self.name, value);
            self.set_boolean(value);
        } else if self.setting_type == SettingType::Toggle {
            debug!("Toggling setting: {:?}", self.name);
            self.toggle();
        } else if self.setting_type == SettingType::Regenerate {
            debug!("Regenerating setting: {:?}", self.name);
            self.regenerate();
        } else {
            debug!("Updating setting: '{:?}' = '{}'", self.name, value);
            self.value = value.to_string();
        }
        self.updated_at = chrono::Utc::now();
    }

    /// Fetch the Setting by Name
    pub async fn get(
        connection: &Connection<'_>,
        name: impl Into<String>,
    ) -> Result<Self, crate::KonarrError> {
        Ok(Self::fetch_by_name(connection, name.into()).await?)
    }

    /// Fetch the Setting by Namespace
    pub async fn get_namespace(
        connection: &Connection<'_>,
        name: impl Into<String>,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        let mut namespace = name.into();
        if !namespace.ends_with('.') {
            namespace.push('.');
        }
        log::debug!("Fetching settings in namespace: `{}%`", namespace);

        Ok(Self::query(
            connection,
            Self::query_select()
                .where_like("name", format!("{}%", namespace))
                .build()?,
        )
        .await?)
    }

    /// Get all Statistics Settings
    pub async fn fetch_statistics(
        connection: &Connection<'_>,
    ) -> Result<Vec<Self>, crate::KonarrError> {
        Ok(Self::query(
            connection,
            Self::query_select()
                .where_eq("setting_type", SettingType::Statistics)
                .build()?,
        )
        .await?)
    }

    /// Fetch the Setting by Name as a Boolean
    pub async fn get_bool(
        connection: &Connection<'_>,
        name: impl Into<Setting>,
    ) -> Result<bool, crate::KonarrError> {
        Ok(Self::fetch_by_name(connection, name.into())
            .await?
            .boolean())
    }

    /// Set and update the Setting
    pub async fn set_update(
        &mut self,
        connection: &Connection<'_>,
        value: impl Into<String>,
    ) -> Result<(), crate::KonarrError> {
        self.set(value.into());
        self.update(connection).await?;
        Ok(())
    }

    /// Toggle the Setting
    pub fn toggle(&mut self) {
        self.value = match self.setting_type {
            SettingType::Toggle => match self.value.as_str() {
                "enabled" => "disabled".to_string(),
                "disabled" => "enabled".to_string(),
                _ => "enabled".to_string(),
            },
            _ => self.value.clone(),
        };
    }

    /// Set the Setting to a Boolean
    pub fn set_boolean(&mut self, value: impl Into<String>) {
        self.value = match value.into().as_str() {
            "true" | "1" | "enabled" => "true".to_string(),
            _ => "false".to_string(),
        }
    }
    /// Get the Setting as a Boolean
    pub fn boolean(&self) -> bool {
        self.value == "true" || self.value == "1" || self.value == "enabled"
    }

    /// Get the Setting as a String
    pub fn string(&self) -> String {
        self.value.clone()
    }

    /// Get the Setting as an Integer
    pub fn integer(&self) -> Result<i64, std::num::ParseIntError> {
        self.value.parse::<i64>()
    }

    /// Regenerate the Setting Value (42 alphanumeric characters)
    pub fn regenerate(&mut self) {
        self.value = geekorm::utils::generate_random_string(42, "kagent_")
    }

    /// Check if security features are enabled
    pub async fn feature_security(connection: &Connection<'_>) -> Result<bool, crate::KonarrError> {
        Self::get_bool(connection, "security").await
    }

    /// Reset the Setting to the default value
    pub async fn reset(&mut self, connection: &Connection<'_>) -> Result<(), crate::KonarrError> {
        if let Some(default) = Self::defaults()
            .iter()
            .find(|(name, _, _)| name == &self.name)
        {
            self.value = default.2.to_string();
            self.update(connection).await?;
            Ok(())
        } else {
            Err(crate::KonarrError::UnknownError(
                "Unknown ServerSettings default value".to_string(),
            ))
        }
    }
}
