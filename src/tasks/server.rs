//! Server Task Module
use geekorm::GeekConnector;

use crate::models::{ServerSettings, Setting};
use crate::{tasks::TaskTrait, tools::Tool};

/// Server Task to handle server-related background tasks
#[derive(Default)]
pub struct ServerTask;

#[async_trait::async_trait]
impl TaskTrait for ServerTask {
    async fn run(&self, database: &geekorm::ConnectionManager) -> Result<(), crate::KonarrError> {
        log::info!("Server Task Running");
        // Implement server-related tasks here
        self.tools(database).await?;

        Ok(())
    }
}

impl ServerTask {
    /// Check and update tools
    pub(crate) async fn tools(
        &self,
        database: &geekorm::ConnectionManager,
    ) -> Result<(), crate::KonarrError> {
        let current_tool =
            ServerSettings::fetch_by_name(&database.acquire().await, Setting::SecurityToolsName)
                .await?;
        log::info!("Current Tool: {}", current_tool.value);

        let mut current_tool_version =
            ServerSettings::fetch_by_name(&database.acquire().await, Setting::SecurityToolsVersion)
                .await?;
        log::info!("Current Tool Version: {}", current_tool_version.value);

        let mut tool = match current_tool.value.as_str() {
            "syft" => crate::tools::Syft::init().await,
            "grype" => crate::tools::Grype::init().await,
            "trivy" => crate::tools::Trivy::init().await,
            _ => {
                log::warn!("Unknown tool configured: {}", current_tool.value);
                return Ok(());
            }
        };

        if let Ok(tool_version_remote) = tool.remote_version().await {
            log::info!("Remote Tool Version: {}", tool_version_remote);

            if current_tool_version.value != tool_version_remote {
                current_tool_version.value = tool_version_remote.clone();
                current_tool_version
                    .update(&database.acquire().await)
                    .await?;
                log::info!("Updated Tool Version to: {}", tool_version_remote);
            }
        }

        Ok(())
    }
}
