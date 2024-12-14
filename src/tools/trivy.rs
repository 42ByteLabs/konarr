//! # Grype

use async_trait::async_trait;
use log::info;

use super::{Tool, ToolConfig};
use crate::KonarrError;

/// Syft Tool
#[derive(Debug)]
pub struct Trivy;

#[async_trait]
impl Tool for Trivy {
    fn init() -> Result<ToolConfig, KonarrError> {
        if let Ok(path) = Self::find("trivy") {
            Ok(ToolConfig::new("trivy", path))
        } else {
            return Err(KonarrError::ToolError("Trivy not found".to_string()));
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
        info!("Running Trivy on image: {}", image);
        let opath = config.output.display().to_string();

        // Run Grype (all layers, output to temp file)
        let output = tokio::process::Command::new(&config.path)
            .args(&[
                "image",
                "--offline-scan",
                "--format",
                "cyclonedx",
                "--output",
                opath.as_str(),
                image.as_str(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(KonarrError::ToolError("Failed to run tool".to_string()));
        }
        if !config.output.exists() {
            return Err(KonarrError::ToolError("No output file".to_string()));
        }
        log::info!("Successfully ran Trivy");

        // Read the output file
        Ok(tokio::fs::read_to_string(config.output.clone()).await?)
    }
}
