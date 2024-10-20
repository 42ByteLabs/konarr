//! Project Request
use serde::{Deserialize, Serialize};

use crate::KonarrError;

use super::{snapshot::KonarrSnapshot, ApiResponse, KonarrClient, Pagination};

/// List of Konarr Projects
pub struct KonarrProjects;

impl KonarrProjects {
    /// List Projects
    pub async fn list(client: &KonarrClient) -> Result<Pagination<KonarrProject>, KonarrError> {
        Ok(client
            .get("/projects")
            .await?
            .json::<Pagination<KonarrProject>>()
            .await?)
    }
    /// Get Project by ID
    pub async fn by_id(
        client: &KonarrClient,
        id: u32,
    ) -> Result<Option<KonarrProject>, KonarrError> {
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
        Ok(client
            .get(&format!("/projects?search={}", name))
            .await?
            .json::<KonarrProject>()
            .await
            .ok())
    }
}

/// Project Request
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct KonarrProject {
    /// Project ID
    #[serde(skip_serializing)]
    pub id: u32,
    /// Project Name
    pub name: String,
    /// Project Type
    pub r#type: String,
    /// Project Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Parent Project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<i32>,
    /// Project Children
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<KonarrProject>>,

    /// Created At
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Latest Snapshot
    pub snapshot: Option<KonarrSnapshot>,
    /// Number of Snapshots
    pub snapshots: u32,
}

impl KonarrProject {
    /// Create a new Project
    pub fn new(name: impl Into<String>, r#type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: r#type.into(),
            ..Default::default()
        }
    }

    /// Create new Project
    pub async fn create(&mut self, client: &KonarrClient) -> Result<Self, KonarrError> {
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
        Ok(client
            .get(&format!("/projects/{}/snapshot", self.id))
            .await?
            .json::<ApiResponse<Self>>()
            .await?)
    }
}
