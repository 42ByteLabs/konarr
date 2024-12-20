//! # Syft

use async_trait::async_trait;
use log::info;

use super::{Tool, ToolConfig};
use crate::KonarrError;

/// Syft Tool
pub struct Syft;

#[async_trait]
impl Tool for Syft {
    async fn init() -> ToolConfig {
        // Initialize Syft (confirm it exists)
        let mut config = if let Ok(path) = Self::find("syft") {
            log::debug!("Found Syft at: {}", path.display());
            ToolConfig::new("syft", path)
        } else {
            ToolConfig {
                name: "syft".to_string(),
                ..Default::default()
            }
        };
        if let Ok(version) = Self::version(&config).await {
            config.version = version;
        }
        if let Ok(ipath) = Self::find("install-syft") {
            config.install_path = Some(ipath.clone());
        }
        log::debug!("Syft Config: {:?}", config);
        config
    }

    async fn run(
        config: &ToolConfig,
        image: impl Into<String> + Send,
    ) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        let image = image.into();

        if let Some(path) = &config.path {
            info!("Running Syft on image: {}", image);
            let output_path = format!("cyclonedx-json={}", config.output.display());

            // Run Syft
            let output = tokio::process::Command::new(&path)
                .args(&["scan", "-o", output_path.as_str(), image.as_str()])
                .output()
                .await?;

            if !output.status.success() {
                return Err(KonarrError::ToolError("Failed to run tool".to_string()));
            }

            // Read the output file
            Ok(config.read_output().await?)
        } else {
            return Err(KonarrError::ToolError("No tool path".to_string()));
        }
    }

    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError> {
        config.github_release("anchore/syft").await
    }
}
