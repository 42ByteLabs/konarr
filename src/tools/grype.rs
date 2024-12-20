//! # Grype

use async_trait::async_trait;
use log::info;

use super::{Tool, ToolConfig};
use crate::KonarrError;

/// Syft Tool
#[derive(Debug)]
pub struct Grype;

#[async_trait]
impl Tool for Grype {
    async fn init() -> ToolConfig {
        let mut config = if let Ok(path) = Self::find("grype") {
            log::debug!("Found Grype at: {}", path.display());
            ToolConfig::new("grype", path)
        } else {
            ToolConfig {
                name: "grype".to_string(),
                ..Default::default()
            }
        };
        if let Ok(version) = Self::version(&config).await {
            config.version = version;
        }
        if let Ok(ipath) = Self::find("install-grype") {
            config.install_path = Some(ipath.clone());
        }

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
            info!("Running Grype on image: {}", image);
            let opath = format!("cyclonedx-json={}", config.output.display());
            log::debug!("Output path: {}", config.output.display());

            log::debug!("Run Grype (all layers, output to temp file)");
            let output = tokio::process::Command::new(&path)
                .args(&["-s", "all-layers", "-o", opath.as_str(), image.as_str()])
                .output()
                .await?;

            if !output.status.success() {
                return Err(KonarrError::ToolError("Failed to run tool".to_string()));
            }
            if !config.output.exists() {
                return Err(KonarrError::ToolError("No output file".to_string()));
            }
            log::info!("Successfully ran Grype");

            // Read the output file
            Ok(config.read_output().await?)
        } else {
            return Err(KonarrError::ToolError("No tool path".to_string()));
        }
    }

    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError> {
        config.github_release("anchore/grype").await
    }
}
