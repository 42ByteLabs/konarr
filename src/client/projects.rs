//! Project Request
use log::debug;
use serde::{Deserialize, Serialize};

use crate::KonarrError;

use super::{
    security::SecuritySummary, snapshot::KonarrSnapshot, ApiResponse, KonarrClient, Pagination,
};

/// List of Konarr Projects
pub struct KonarrProjects;

impl KonarrProjects {
    /// List Projects
    pub async fn list(client: &KonarrClient) -> Result<Pagination<KonarrProject>, KonarrError> {
        debug!("Listing Projects");
        match client
            .get("/projects")
            .await?
            .json::<ApiResponse<Pagination<KonarrProject>>>()
            .await?
        {
            ApiResponse::Ok(pagination) => Ok(pagination),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// List Top Projects
    pub async fn list_top(client: &KonarrClient) -> Result<Pagination<KonarrProject>, KonarrError> {
        debug!("Listing Top Projects");
        match client
            .get("/projects?top=true")
            .await?
            .json::<ApiResponse<Pagination<KonarrProject>>>()
            .await?
        {
            ApiResponse::Ok(pagination) => Ok(pagination),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Search Projects
    pub async fn search(
        client: &KonarrClient,
        search: impl Into<String>,
    ) -> Result<Pagination<KonarrProject>, KonarrError> {
        let search = search.into();
        debug!("Searching Projects: {}", search);
        match client
            .get(&format!("/projects?search={}", search))
            .await?
            .json::<ApiResponse<Pagination<KonarrProject>>>()
            .await?
        {
            ApiResponse::Ok(pagination) => Ok(pagination),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Get Project by ID
    pub async fn by_id(
        client: &KonarrClient,
        id: u32,
    ) -> Result<Option<KonarrProject>, KonarrError> {
        debug!("Getting Project by ID: {}", id);
        Ok(client
            .get(&format!("/projects/{}", id))
            .await?
            .json::<KonarrProject>()
            .await
            .ok())
    }
    /// Get Project by Name
    pub async fn by_name(
        client: &KonarrClient,
        name: &str,
    ) -> Result<Option<KonarrProject>, KonarrError> {
        debug!("Getting Project by Name: {}", name);
        let search = Self::search(client, name).await?;

        for result in search.data {
            if result.title == name || result.name == name {
                return Ok(Some(result));
            }
        }
        return Ok(None);
    }
}

/// Project Request
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KonarrProject {
    /// Project ID
    #[serde(skip_serializing)]
    pub id: u32,
    /// Project Name
    pub name: String,
    /// Project title
    #[serde(skip_serializing)]
    pub title: String,
    /// Project Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Project Type
    #[serde(rename = "type")]
    pub project_type: String,
    /// Project Status
    #[serde(skip_serializing)]
    pub status: Option<bool>,

    /// Latest Snapshot
    #[serde(skip_serializing)]
    pub snapshot: Option<KonarrSnapshot>,
    /// Number of Snapshots
    #[serde(skip_serializing)]
    pub snapshots: u32,

    /// Security
    #[serde(skip_serializing)]
    pub security: Option<SecuritySummary>,

    /// Parent Project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<i32>,

    /// Project Children
    #[serde(skip_serializing)]
    pub children: Option<Vec<KonarrProject>>,

    /// Created At
    #[serde(skip_serializing)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl KonarrProject {
    /// Create a new Project
    pub fn new(name: impl Into<String>, r#type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            project_type: r#type.into(),
            ..Default::default()
        }
    }

    /// Create new Project
    pub async fn create(&mut self, client: &KonarrClient) -> Result<Self, KonarrError> {
        debug!("Creating Project: {}", self.name);
        match client
            .post("/projects", &self)
            .await?
            .json::<ApiResponse<Self>>()
            .await?
        {
            ApiResponse::Ok(project) => {
                *self = project;
                Ok(self.clone())
            }
            ApiResponse::Error(err) => Err(err.into()),
        }
    }

    /// Get Project by ID
    pub async fn get(&mut self, client: &KonarrClient) -> Result<ApiResponse<Self>, KonarrError> {
        debug!("Getting Project by ID: {}", self.id);
        match client
            .get(&format!("/projects/{}", self.id))
            .await?
            .json::<ApiResponse<Self>>()
            .await?
        {
            ApiResponse::Ok(project) => {
                *self = project;
                Ok(ApiResponse::Ok(self.clone()))
            }
            ApiResponse::Error(err) => Ok(ApiResponse::Error(err)),
        }
    }

    /// Get Project Snapshot
    pub async fn get_snapshot(
        &self,
        client: &KonarrClient,
    ) -> Result<ApiResponse<Self>, KonarrError> {
        debug!("Getting Project Snapshot: {}", self.id);
        Ok(client
            .get(&format!("/projects/{}/snapshot", self.id))
            .await?
            .json::<ApiResponse<Self>>()
            .await?)
    }
}
