//! # Konarr Rust Client Library
//!
//! This library provides a client for interacting with the Konarr API.
//!
//! ```no_run
//! # use anyhow::Result;
//! use konarr::KonarrClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!    let client = KonarrClient::init()
//!         .base("https://localhost:8000")?
//!         .build()?;
//!
//!     Ok(())
//! }
//! ```

use log::{debug, info};
use server::User;
use url::Url;

pub mod projects;
pub mod security;
pub mod server;
pub mod snapshot;

pub use server::ServerInfo;

use crate::KonarrError;

/// Pagination Response
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Pagination<T>
where
    T: serde::Serialize + Send,
{
    /// Data Response
    pub data: Vec<T>,
    /// Pages
    pub pages: u64,
    /// Total
    pub total: u64,
}

/// API Error
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiError {
    /// Error Message
    pub message: String,
    /// Error Details
    pub details: Option<String>,
    /// Error Status Code
    pub status: u16,
}

/// API Response
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T>
where
    T: serde::Serialize + Send,
{
    /// Ok Response
    Ok(T),
    /// Error Response
    Error(ApiError),
}

/// Konarr REST Client
#[derive(Debug, Clone)]
pub struct KonarrClient {
    /// Base URL
    url: Url,
    /// Web Client
    client: reqwest::Client,
    /// Web Server Token
    token: Option<String>,
}

impl KonarrClient {
    /// New Konarr Client
    pub fn new(url: impl Into<Url>) -> Self {
        let url = url.into();
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        log::debug!("Setting up Konarr Client for {}", url);

        Self {
            client,
            url,
            token: None,
        }
    }

    /// Initialize a new Konarr Client Builder
    pub fn init() -> KonarrClientBuilder {
        KonarrClientBuilder::new()
    }

    /// Get the URL of the client
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get the Base URL + Path
    pub(crate) fn base(&self, path: &str) -> Result<Url, url::ParseError> {
        let base = self.url.path().trim_end_matches('/');
        self.url.join(&format!("{}{}", base, path))
    }

    /// Check to see if the client is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.server()
            .await
            .map(|svr| svr.user.is_some())
            .unwrap_or(false)
    }

    /// Get Server Information
    pub async fn server(&self) -> Result<ServerInfo, crate::KonarrError> {
        debug!("Getting Server Information");
        self.get("/").await?.json().await.map_err(KonarrError::from)
    }

    /// Get Projects
    pub async fn get_projects(
        &self,
    ) -> Result<Pagination<projects::KonarrProject>, crate::KonarrError> {
        projects::KonarrProjects::list(self).await
    }

    /// Get the User Information
    pub async fn user(&self) -> Result<Option<User>, crate::KonarrError> {
        debug!("Getting User Information");
        Ok(self.server().await?.user)
    }

    /// Login to Konarr Server
    pub async fn login(
        &mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<(), KonarrError> {
        let response = self
            .post(
                "/auth/login",
                &serde_json::json!({
                    "username": username.into(),
                    "password": password.into(),
                }),
            )
            .await?;

        if response.status().is_success() {
            info!("Login Successful");
            Ok(())
        } else {
            Err(KonarrError::UnknownError("Login Failed".to_string()))
        }
    }

    /// Client GET Request
    pub async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(self.base(path).unwrap())
            .header("Authorization", self.token.clone().unwrap_or_default())
            .send()
            .await
    }
    /// Client POST Request
    pub async fn post<T>(&self, path: &str, json: T) -> Result<reqwest::Response, reqwest::Error>
    where
        T: serde::Serialize + Send,
    {
        self.client
            .post(self.base(path).unwrap())
            .header("Authorization", self.token.clone().unwrap_or_default())
            .json(&json)
            .send()
            .await
    }
    /// Client PATCH Request
    pub async fn patch<T>(&self, path: &str, json: T) -> Result<reqwest::Response, reqwest::Error>
    where
        T: serde::Serialize + Send,
    {
        self.client
            .patch(self.base(path).unwrap())
            .header("Authorization", self.token.clone().unwrap_or_default())
            .json(&json)
            .send()
            .await
    }
    /// Client DELETE Request
    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .delete(self.base(path).unwrap())
            .header("Authorization", self.token.clone().unwrap_or_default())
            .send()
            .await
    }
}

/// Konarr Client Builder
#[derive(Debug, Default)]
pub struct KonarrClientBuilder {
    url: Option<Url>,
    token: Option<String>,
}

impl KonarrClientBuilder {
    /// Create a new Konarr Client Builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the Base URL
    pub fn base(mut self, url: impl Into<String>) -> Result<Self, crate::KonarrError> {
        self.url = Some(Url::parse(&url.into())?);
        Ok(self)
    }

    /// Set the API Token
    pub fn token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    /// Build the Konarr Client
    pub fn build(self) -> Result<KonarrClient, KonarrError> {
        if let Some(url) = self.url {
            let client = reqwest::Client::builder()
                .cookie_store(true)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap();

            Ok(KonarrClient {
                client,
                url,
                token: self.token,
            })
        } else {
            Err(KonarrError::UnknownError("Base URL not set".to_string()))
        }
    }
}
