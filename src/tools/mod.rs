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
        let version = String::from_utf8(output.stdout)?.replace(&config.name, "");
        Ok(version.trim().to_string())
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

    /// Get the remote version of the Tool
    async fn remote_version<'a>(config: &'a mut ToolConfig) -> Result<String, KonarrError>
    where
        Self: Sized;

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
                let mut grype = Grype::init()?;
                log::info!("Running Grype :: {}", Grype::version(&mut grype).await?);
                Grype::run(&grype, image).await
            }
            "syft" => {
                let mut syft = Syft::init()?;
                log::info!("Running Syft :: {}", Syft::version(&mut syft).await?);
                Syft::run(&syft, image).await
            }
            "trivy" => {
                let mut trivy = Trivy::init()?;
                log::info!("Running Trivy :: {}", Trivy::version(&mut trivy).await?);
                Trivy::run(&trivy, image).await
            }
            _ => Err(KonarrError::ToolError(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    } else {
        log::info!("No tool specified, trying to find a tool");

        if let Ok(mut grype) = Grype::init() {
            log::info!("Running Grype :: {}", Grype::version(&mut grype).await?);
            Grype::run(&grype, image).await
        } else if let Ok(mut trivy) = Trivy::init() {
            log::info!("Running Trivy :: {}", Trivy::version(&mut trivy).await?);
            Trivy::run(&trivy, image).await
        } else if let Ok(mut syft) = Syft::init() {
            log::info!("Running Syft :: {}", Syft::version(&mut syft).await?);
            Syft::run(&syft, image).await
        } else {
            Err(KonarrError::ToolError("No tools found".to_string()))
        }
    }
}

/// Tool Configuration
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,
    /// Version of the tool
    pub version: String,
    /// Remote version of the tool
    pub remote_version: Option<String>,
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
            version: String::new(),
            remote_version: None,
            path,
            output,
        }
    }

    /// Get the list of available tools
    pub async fn tools() -> Result<Vec<ToolConfig>, KonarrError> {
        let mut tools = vec![];

        if let Ok(mut grype) = Grype::init() {
            grype.version = Grype::version(&grype).await?;
            tools.push(grype);
        }
        if let Ok(mut syft) = Syft::init() {
            syft.version = Syft::version(&syft).await?;
            tools.push(syft);
        }
        if let Ok(mut trivy) = Trivy::init() {
            trivy.version = Trivy::version(&trivy).await?;
            tools.push(trivy);
        }

        Ok(tools)
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

    /// Get the remote version of the Tool from GitHub releases
    ///
    /// https://docs.github.com/en/rest/releases/releases?apiVersion=2022-11-28#get-the-latest-release
    pub(crate) async fn github_release(
        &mut self,
        repository: impl Into<String>,
    ) -> Result<String, KonarrError> {
        let repository = repository.into();
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            repository
        );
        log::debug!("Getting release from GitHub: {}", url);
        // Accept header: application/vnd.github.v3+json
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        let client = reqwest::Client::builder()
            .user_agent(format!("Konarr/{}", crate::KONARR_VERSION))
            .default_headers(headers)
            .build()?;
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            log::error!("Failed to get release from GitHub: {}", repository);
            return Err(KonarrError::ToolError(format!(
                "Failed to get release: {}",
                response.status()
            )));
        }
        let json: serde_json::Value = response.json().await?;
        let version = json["tag_name"]
            .as_str()
            .ok_or(KonarrError::ToolError("No tag_name".to_string()))?;
        self.remote_version = Some(version.to_string());
        Ok(version.to_string())
    }
}
