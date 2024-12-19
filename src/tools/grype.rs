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
    fn init() -> Result<ToolConfig, KonarrError> {
        if let Ok(path) = Self::find("grype") {
            log::debug!("Found Grype at: {}", path.display());
            Ok(ToolConfig::new("grype", path))
        } else {
            return Err(KonarrError::ToolError("Grype not found".to_string()));
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

        info!("Running Grype on image: {}", image);
        let opath = format!("cyclonedx-json={}", config.output.display());
        log::debug!("Output path: {}", config.output.display());

        log::debug!("Run Grype (all layers, output to temp file)");
        let output = tokio::process::Command::new(&config.path)
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
    }

    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError> {
        config.github_release("anchore/grype").await
    }
}
