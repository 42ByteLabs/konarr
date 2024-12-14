//! # Grype

use async_trait::async_trait;
use log::info;
use std::path::PathBuf;

use super::Tool;
use crate::KonarrError;

/// Syft Tool
#[derive(Debug)]
pub struct Grype {
    path: PathBuf,
}

#[async_trait]
impl Tool for Grype {
    async fn init() -> Result<Self, KonarrError> {
        // Initialize Grype (confirm it exists)
        Ok(Grype {
            path: Self::find()?,
        })
    }

    async fn run(&self, image: impl Into<String> + Send) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        let image = image.into();
        info!("Running Grype on image: {}", image);
        let output_path = format!("cyclonedx-json={}", self.temp_path());
        // Run Grype (all layers, output to temp file)
        let output = tokio::process::Command::new(&self.path)
            .args(&[
                "-s",
                "all-layers",
                "-o",
                output_path.as_str(),
                image.as_str(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(KonarrError::ToolError("Failed to run tool".to_string()));
        }

        // Read the output file
        Ok(tokio::fs::read_to_string(self.temp_path()).await?)
    }
}

impl Grype {
    /// Find the Grype binary
    pub fn find() -> Result<PathBuf, KonarrError> {
        let locations = vec![
            "/usr/local/bin/grype",
            "/usr/bin/grype",
            "/bin/grype",
            "/snap/bin/grype",
        ];
        for loc in locations {
            if std::path::Path::new(loc).exists() {
                info!("Found Grype at: {}", loc);
                return Ok(PathBuf::from(loc));
            }
        }
        return Err(KonarrError::ToolError("Grype not found".to_string()));
    }

    fn temp_path(&self) -> String {
        String::from("grype-output.json")
    }
}
