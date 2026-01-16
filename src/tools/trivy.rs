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
    async fn init() -> ToolConfig {
        let mut config = if let Ok(path) = Self::find("trivy") {
            log::debug!("Found Trivy at: {}", path.display());
            ToolConfig::new("trivy", path)
        } else {
            ToolConfig {
                name: "trivy".to_string(),
                ..Default::default()
            }
        };
        if let Ok(version) = Self::version(&config).await {
            config.version = version;
        }
        if let Ok(ipath) = Self::find("install-trivy") {
            config.install_path = Some(ipath.clone());
        }

        config
    }

    async fn version(config: &ToolConfig) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        if let Some(path) = &config.path {
            let output = tokio::process::Command::new(path)
                .args(["--version"])
                .output()
                .await?;
            if !output.status.success() {
                return Err(KonarrError::ToolError("Failed to get version".to_string()));
            }
            let data = String::from_utf8(output.stdout)?;
            // Read the first line of the output
            let first_line = data.lines().next().unwrap_or_default();
            let version = first_line.replace("Version: ", "");
            Ok(version)
        } else {
            Err(KonarrError::ToolError("No tool path".to_string()))
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
        if let Some(path) = &config.path {
            info!("Running Trivy on image: {}", image);
            let opath = config.output.display().to_string();

            // Run Grype (all layers, output to temp file)
            let output = tokio::process::Command::new(path)
                .args([
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
            Ok(config.read_output().await?)
        } else {
            return Err(KonarrError::ToolError("No tool path".to_string()));
        }
    }

    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError> {
        config.github_release("aquasecurity/trivy").await
    }
}
