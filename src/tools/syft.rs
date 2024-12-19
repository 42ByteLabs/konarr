//! # Syft

use async_trait::async_trait;
use log::info;

use super::{Tool, ToolConfig};
use crate::KonarrError;

/// Syft Tool
pub struct Syft;

#[async_trait]
impl Tool for Syft
where
    Self: Sized,
{
    fn init() -> Result<ToolConfig, KonarrError> {
        // Initialize Syft (confirm it exists)
        if let Ok(path) = Self::find("syft") {
            Ok(ToolConfig::new("syft", path))
        } else {
            return Err(KonarrError::ToolError("Syft not found".to_string()));
        }
    }

    async fn run(
        config: &ToolConfig,
        image: impl Into<String> + Send,
    ) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        let image = image.into();

        info!("Running Syft on image: {}", image);
        let output_path = format!("cyclonedx-json={}", config.output.display());

        // Run Syft
        let output = tokio::process::Command::new(&config.path)
            .args(&["scan", "-o", output_path.as_str(), image.as_str()])
            .output()
            .await?;

        if !output.status.success() {
            return Err(KonarrError::ToolError("Failed to run tool".to_string()));
        }

        // Read the output file
        Ok(config.read_output().await?)
    }

    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError> {
        config.github_release("anchore/syft").await
    }
}
