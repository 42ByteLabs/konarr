//! # Server Settings Model
use geekorm::prelude::*;
use log::{debug, info};
use serde::{Deserialize, Serialize};

/// Setting Type
#[derive(Data, Debug, Default, Clone, PartialEq)]
pub enum SettingType {
    /// Toggle (enabled/disabled)
    Toggle,
    /// Regenerate value (e.g. API Key)
    Regenerate,

    /// Boolean
    Boolean,
    /// String
    #[default]
    String,
    /// Integer
    Integer,
    /// Float
    Float,

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
    pub name: String,

    /// Setting Type
    pub setting_type: SettingType,

    /// Setting Value
    #[geekorm(not_null)]
    pub value: String,

    /// Updated At Datetime
    #[geekorm(new = "chrono::Utc::now()", on_update = "chrono::Utc::now()")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ServerSettings {
    /// Initialize the Server Settings Table
    pub async fn init<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        ServerSettings::create_table(connection).await?;

        let agent_key = geekorm::utils::generate_random_string(43, "kagent_");
        let settings = vec![
            // Registration Settings
            ("registration", SettingType::Toggle, "enabled"),
            // If we are already initialized
            ("initialized", SettingType::Boolean, "false"),
            // Agent Settings
            ("agent", SettingType::Toggle, "disabled"),
            ("agent.key", SettingType::Regenerate, agent_key.as_str()),
            // Security Features
            ("security", SettingType::Toggle, "disabled"),
            ("security.polling", SettingType::Delete, "disabled"),
            // Alerts Count Caching
            ("security.alerts.total", SettingType::Integer, "0"),
            ("security.alerts.critical", SettingType::Integer, "0"),
            ("security.alerts.high", SettingType::Integer, "0"),
            ("security.alerts.medium", SettingType::Integer, "0"),
            ("security.alerts.low", SettingType::Integer, "0"),
            ("security.alerts.infomational", SettingType::Integer, "0"),
            ("security.alerts.malware", SettingType::Integer, "0"),
            ("security.alerts.unmaintained", SettingType::Integer, "0"),
            ("security.alerts.other", SettingType::Integer, "0"),
            // Tools Settings
            ("security.tools.alerts", SettingType::Toggle, "enabled"),
            // Advisories Settings
            ("security.advisories", SettingType::Toggle, "disabled"),
            ("security.advisories.pull", SettingType::Toggle, "disabled"),
            (
                "security.advisories.version",
                SettingType::String,
                "Unknown",
            ),
            (
                "security.advisories.updated",
                SettingType::String,
                "Unknown",
            ),
            (
                "security.advisories.polling",
                SettingType::Toggle,
                "disabled",
            ),
        ];

        for (name, typ, value) in settings {
            match ServerSettings::fetch_by_name(connection, name).await {
                Ok(mut setting) => {
                    if setting.setting_type == SettingType::Delete {
                        debug!("Deleting setting: {}", name);
                        // setting.delete(connection).await?;
                    } else {
                        // Update setting type in case it has changed in newer versions
                        if setting.setting_type != typ {
                            debug!("Updating setting: {}", name);
                            setting.setting_type = typ;
                            setting.update(connection).await?;
                        }
                    }
                }
                Err(_) => {
                    let mut setting = ServerSettings::new(name, typ, value);
                    setting.save(connection).await?;
                }
            };
        }

        Ok(())
    }

    /// Set the Setting
    pub fn set(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.setting_type == SettingType::Boolean {
            info!("Setting boolean: {} = {}", self.name, value);
            self.set_boolean(value);
        } else if self.setting_type == SettingType::Toggle {
            info!("Toggling setting: {}", self.name);
            self.toggle();
        } else if self.setting_type == SettingType::Regenerate {
            info!("Regenerating setting: {}", self.name);
            self.regenerate();
        } else {
            info!("Updating setting: '{}' = '{}'", self.name, value);
            self.value = value.to_string();
        }
        self.updated_at = chrono::Utc::now();
    }

    /// Fetch the Setting by Name
    pub async fn get<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Self::fetch_by_name(connection, name.into()).await?)
    }

    /// Fetch the Setting by Namespace
    pub async fn get_namespace<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
    ) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut namespace = name.into();
        if !namespace.ends_with('.') {
            namespace.push('.');
        }

        Ok(Self::query(
            connection,
            Self::query_select()
                .where_like("name", format!("{}%", namespace))
                .build()?,
        )
        .await?)
    }

    /// Fetch the Setting by Name as a Boolean
    pub async fn get_bool<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
    ) -> Result<bool, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Self::fetch_by_name(connection, name.into())
            .await?
            .boolean())
    }

    /// Set and update the Setting
    pub async fn set_update<'a, T>(
        &mut self,
        connection: &'a T,
        value: impl Into<String>,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
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

    /// Regenerate the Setting Value (42 alphanumeric characters)
    pub fn regenerate(&mut self) {
        self.value = geekorm::utils::generate_random_string(42, "kagent_")
    }
}
