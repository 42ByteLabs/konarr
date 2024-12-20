//! Tools to analyze the BOM of a container image
use std::{fmt::Display, path::PathBuf};

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
    async fn init() -> ToolConfig
    where
        Self: Sized;

    /// Get the version of the Tool
    async fn version(config: &ToolConfig) -> Result<String, KonarrError>
    where
        Self: Sized,
    {
        if let Some(path) = &config.path {
            let output = tokio::process::Command::new(path)
                .args(&["--version"])
                .output()
                .await?;
            if !output.status.success() {
                return Err(KonarrError::ToolError("Failed to get version".to_string()));
            }
            let version = String::from_utf8(output.stdout)?.replace(&config.name, "");
            Ok(version.trim().to_string())
        } else {
            Err(KonarrError::ToolError("No tool path".to_string()))
        }
    }

    /// Find the Tool
    fn find(binary: &str) -> Result<PathBuf, KonarrError>
    where
        Self: Sized,
    {
        let locations = vec![
            "/usr/local/toolcache",
            "/usr/local/bin/",
            "/usr/bin/",
            "/bin/",
            "/snap/bin/",
        ];
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

    let mut tools = ToolConfig::tools().await?;

    if let Some(tool_name) = &config.agent.tool {
        log::info!("Using tool: {}", tool_name);

        let tool = tools
            .iter_mut()
            .find(|t| t.name.to_lowercase() == tool_name.to_lowercase())
            .ok_or(KonarrError::ToolError(format!(
                "Tool not found: {}",
                tool_name
            )))?;

        if !tool.is_available() {
            if config.agent.tool_auto_install {
                log::info!("Tool is not available, trying to install: {}", tool.name);
                tool.install().await?;
            } else {
                log::info!("Tool is not available: {}", tool.name);
                return Err(KonarrError::ToolError(format!(
                    "Tool not available: {}",
                    tool_name
                )));
            }
        }
        log::info!("Tool is available: {}", tool);

        log::info!("Running tool: {}", tool);
        tool.run(image).await
    } else {
        log::info!("No tool specified, trying to find a tool");

        for tool in tools.iter() {
            if tool.is_available() {
                log::info!("Running tool: {}", tool);
                return tool.run(image).await;
            }
        }

        Err(KonarrError::ToolError("No tools found".to_string()))
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
    pub path: Option<PathBuf>,
    /// Install Script path
    pub install_path: Option<PathBuf>,
    /// Output path for the SBOM file
    pub output: PathBuf,
}

const TOOLCACHE_DIRS: &[&str] = &["/usr/local/toolcache", "/usr/local/bin/"];

impl ToolConfig {
    /// New Tool Configuration
    pub fn new(name: &str, path: PathBuf) -> Self {
        Self {
            name: name.to_string(),
            path: Some(path),
            ..Default::default()
        }
    }

    /// Get the list of available tools
    pub async fn tools() -> Result<Vec<ToolConfig>, KonarrError> {
        log::debug!("Getting list of available tools");
        let mut tools = vec![];

        tools.push(Grype::init().await);
        tools.push(Syft::init().await);
        tools.push(Trivy::init().await);

        log::debug!("Number of tools found: {}", tools.len());
        Ok(tools)
    }

    /// Check if the Tool is available
    pub fn is_available(&self) -> bool {
        if self.path.is_some() && !self.version.is_empty() {
            true
        } else {
            false
        }
    }

    /// Run the Tool
    pub async fn run(&self, image: impl Into<String> + Send) -> Result<String, KonarrError> {
        match self.name.as_str() {
            "grype" => {
                return Grype::run(&self, image).await;
            }
            "syft" => {
                return Syft::run(&self, image).await;
            }
            "trivy" => {
                return Trivy::run(&self, image).await;
            }
            _ => panic!("Tool not implemented"),
        }
    }

    /// Find the Tool
    pub fn find(&self) -> Result<PathBuf, KonarrError> {
        log::debug!("Finding tool: {}", self.name);
        match self.name.as_str() {
            "grype" => {
                return Grype::find("grype");
            }
            "syft" => {
                return Syft::find("syft");
            }
            "trivy" => {
                return Trivy::find("trivy");
            }
            _ => panic!("Tool not implemented"),
        }
    }

    /// Get the version of the Tool
    pub async fn version(&self) -> Result<String, KonarrError> {
        match self.name.as_str() {
            "grype" => {
                return Grype::version(&self).await;
            }
            "syft" => {
                return Syft::version(&self).await;
            }
            "trivy" => {
                return Trivy::version(&self).await;
            }
            _ => panic!("Tool not implemented"),
        }
    }

    /// Install the Tool
    pub async fn install(&mut self) -> Result<(), KonarrError> {
        if let Some(ipath) = &self.install_path {
            log::debug!("Running install script: {}", ipath.display());

            let output: PathBuf = TOOLCACHE_DIRS
                .iter()
                .map(|d| PathBuf::from(d))
                .find(|d| {
                    d.is_dir() && d.exists() && !d.metadata().unwrap().permissions().readonly()
                })
                .unwrap_or_else(|| std::env::temp_dir());
            log::debug!("Toolcache directory: {}", output.display());

            tokio::process::Command::new("sh")
                .arg(ipath)
                .args(&["-b", output.display().to_string().as_str()])
                .output()
                .await
                .map_err(|e| {
                    KonarrError::ToolError(format!("Failed to run install script: {}", e))
                })?;
            log::info!("Successfully installed {}", self.name);

            self.path = self.find().ok();
            if let Ok(version) = self.version().await {
                self.version = version;
            }
            Ok(())
        } else {
            Err(KonarrError::ToolError(
                "No install script found".to_string(),
            ))
        }
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

impl Default for ToolConfig {
    fn default() -> Self {
        let temp_path = std::env::temp_dir().join("konarr");
        let output = temp_path.join(format!("unknown-{}.json", chrono::Utc::now().timestamp()));
        if let Some(parent) = output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Self {
            name: "".to_string(),
            version: "".to_string(),
            path: None,
            install_path: None,
            remote_version: None,
            output,
        }
    }
}

impl Display for ToolConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_available() {
            write!(f, "{} ({})", self.name, self.version)
        } else {
            write!(f, "{} (Not installed)", self.name)
        }
    }
}
