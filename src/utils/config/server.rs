use super::ServerConfig;
use crate::{error::KonarrError, utils::rand::generate_random_string};
use base64::Engine;
use url::Url;

impl ServerConfig {
    /// Set Instance from URL
    pub fn set_instance(&mut self, instance: &String) -> Result<(), KonarrError> {
        let url: Url = Url::parse(instance)?;

        self.scheme = Some(url.scheme().to_string());
        if let Some(host) = url.host_str() {
            self.domain = Some(host.to_string());
        }
        if let Some(port) = url.port_or_known_default() {
            self.port = Some(port as i32);
        }
        Ok(())
    }

    /// Get the Server URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.server.url().unwrap();
    ///
    /// assert_eq!(url.as_str(), "http://localhost:9000/");
    /// ```
    pub fn url(&self) -> Result<Url, KonarrError> {
        let scheme = if let Some(scheme) = &self.scheme {
            scheme.clone()
        } else {
            "http".to_string()
        };

        let port = self
            .port
            .unwrap_or_else(|| if scheme.as_str() == "http" { 9000 } else { 443 });

        let url = Url::parse(&format!(
            "{}://{}:{}",
            scheme,
            self.domain.clone().unwrap_or("localhost".to_string()),
            port
        ))?;
        if url.scheme() != "https" {
            log::warn!("Using insecure scheme: {}", url.scheme());
        }
        Ok(url)
    }

    /// Get the Server API URL
    ///
    /// ```rust
    /// let config = konarr::Config::default();
    /// let url = config.server.api_url().unwrap();
    ///
    /// assert_eq!(url.as_str(), "http://localhost:9000/api");
    /// ```
    pub fn api_url(&self) -> Result<Url, KonarrError> {
        let url = self.url()?;
        let api_base = self.api.clone().unwrap_or_else(|| "/api".to_string());
        Ok(url.join(api_base.as_str())?)
    }

    /// Generate a base64 encoded secret
    pub fn generate_secret() -> String {
        log::debug!("Generating Server Secret...");
        let secret = generate_random_string(32);
        let secret64 = base64::engine::general_purpose::STANDARD.encode(secret);
        secret64
    }
}
