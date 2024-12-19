//! Tools to analyze the BOM of a container image
use std::path::PathBuf;

use crate::{Config, KonarrError};
use async_trait::async_trait;

pub mod grype;
pub mod syft;
pub mod trivy;

pub use grype::Grype;
pub use syft::Syft;
pub use trivy::Trivy;

/// Tool Trait
#[async_trait]
pub trait Tool {
    /// Initialize the Tool
    fn init() -> Result<ToolConfig, KonarrError>
    where
        Self: Sized;

    /// Get the version of the Tool
    async fn version(config: &ToolConfig) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        let output = tokio::process::Command::new(&config.path)
            .args(&["--version"])
            .output()
            .await?;
        if !output.status.success() {
            return Err(KonarrError::ToolError("Failed to get version".to_string()));
        }
        let version = String::from_utf8(output.stdout)?;
        Ok(version)
    }

    /// Find the Tool
    fn find(binary: &str) -> Result<PathBuf, KonarrError>
    where
        Self: Sized,
    {
        let locations = vec!["/usr/local/bin/", "/usr/bin/", "/bin/", "/snap/bin/"];
        for loc in locations {
            let bin = PathBuf::from(loc).join(binary);
            if bin.exists() {
                return Ok(bin);
            }
        }
        Err(KonarrError::ToolError(format!(
            "Can not find tool {}",
            binary
        )))
    }

    /// Run the Tool
    async fn run(
        config: &ToolConfig,
        image: impl Into<String> + Send,
    ) -> Result<String, KonarrError>
    where
        Self: Sized;
}

/// Gets a list of available tools
pub async fn run(config: &Config, image: impl Into<String>) -> Result<String, KonarrError> {
    let image = image.into();

    if let Some(tool_name) = &config.agent.tool {
        log::info!("Using tool: {}", tool_name);
        match tool_name.as_str() {
            "grype" => {
                let grype = Grype::init()?;
                log::info!("Running Grype :: {}", Grype::version(&grype).await?);
                Grype::run(&grype, image).await
            }
            "syft" => {
                let syft = Syft::init()?;
                log::info!("Running Syft :: {}", Syft::version(&syft).await?);
                Syft::run(&syft, image).await
            }
            "trivy" => {
                let trivy = Trivy::init()?;
                log::info!("Running Trivy :: {}", Trivy::version(&trivy).await?);
                Trivy::run(&trivy, image).await
            }
            _ => Err(KonarrError::ToolError(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    } else {
        log::info!("No tool specified, trying to find a tool");

        if let Ok(grype) = Grype::init() {
            log::info!("Running Grype :: {}", Grype::version(&grype).await?);
            Grype::run(&grype, image).await
        } else if let Ok(trivy) = Trivy::init() {
            log::info!("Running Trivy :: {}", Trivy::version(&trivy).await?);
            Trivy::run(&trivy, image).await
        } else if let Ok(syft) = Syft::init() {
            log::info!("Running Syft :: {}", Syft::version(&syft).await?);
            Syft::run(&syft, image).await
        } else {
            Err(KonarrError::ToolError("No tools found".to_string()))
        }
    }
}

/// Gets a list of available tools
pub async fn get_available_tools() -> Result<Vec<String>, KonarrError> {
    let mut tools = vec![];
    if let Ok(grype) = Grype::init() {
        tools.push(grype.name);
    }
    if let Ok(syft) = Syft::init() {
        tools.push(syft.name);
    }
    if let Ok(trivy) = Trivy::init() {
        tools.push(trivy.name);
    }
    Ok(tools)
}

/// Tool Configuration
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,
    /// Tool path
    pub path: PathBuf,
    /// Output path for the SBOM file
    pub output: PathBuf,
}

impl ToolConfig {
    /// New Tool Configuration
    pub fn new(name: &str, path: PathBuf) -> Self {
        let temp_path = std::env::temp_dir().join("konarr");
        let output = temp_path.join(format!("{}-{}.json", name, chrono::Utc::now().timestamp()));
        if let Some(parent) = output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Self {
            name: name.to_string(),
            path,
            output,
        }
    }

    /// Run the Tool
    pub async fn run<T>(&self, image: impl Into<String> + Send) -> Result<String, KonarrError>
    where
        T: Tool,
    {
        T::run(self, image).await
    }

    /// Read the output file
    pub async fn read_output(&self) -> Result<String, KonarrError> {
        tokio::fs::read_to_string(&self.output)
            .await
            .map_err(|e| KonarrError::ToolError(format!("Failed to read output: {}", e)))
    }
}
