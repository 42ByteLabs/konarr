//! # Syft

use async_trait::async_trait;
use log::info;
use std::path::PathBuf;

use super::Tool;
use crate::KonarrError;

/// Syft Tool
#[derive(Debug)]
pub struct Syft {
    path: PathBuf,
}

#[async_trait]
impl Tool for Syft {
    type Results = serde_json::Value;

    async fn init() -> Result<Self, KonarrError> {
        // Initialize Syft (confirm it exists)
        Ok(Syft {
            path: Self::find()?,
        })
    }

    async fn run(&self, image: impl Into<String> + Send) -> Result<Self::Results, KonarrError> {
        let image = image.into();
        info!("Running Syft on image: {}", image);
        let output_path = format!("cyclonedx-json={}", self.temp_path());
        // Run Syft
        let output = tokio::process::Command::new(&self.path)
            .args(&["scan", "-o", output_path.as_str(), image.as_str()])
            .output()
            .await?;

        if !output.status.success() {
            return Err(KonarrError::ToolError("Failed to run tool".to_string()));
        }

        // Read the output file
        let output = std::fs::read_to_string(self.temp_path())?;
        Ok(serde_json::from_str(&output)?)
    }
}

impl Syft {
    /// Find the Syft binary
    pub fn find() -> Result<PathBuf, KonarrError> {
        let locations = vec![
            "/usr/local/bin/syft",
            "/usr/bin/syft",
            "/bin/syft",
            "/snap/bin/syft",
        ];
        for loc in locations {
            if std::path::Path::new(loc).exists() {
                info!("Found Syft at: {}", loc);
                return Ok(PathBuf::from(loc));
            }
        }
        return Err(KonarrError::ToolError("Syft not found".to_string()));
    }

    fn temp_path(&self) -> String {
        String::from("syft-output.json")
    }
}
