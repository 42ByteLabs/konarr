//! Tools to analyze the BOM of a container image
use crate::{client::snapshot::KonarrSnapshot, KonarrClient, KonarrError};
use async_trait::async_trait;
use serde::Serialize;

pub mod syft;

/// Tool Trait
#[async_trait]
pub trait Tool {
    /// The Results of the Tool
    type Results;

    /// Initialize the Tool
    async fn init() -> Result<Self, KonarrError>
    where
        Self: Sized;

    /// Run the Tool
    async fn run(&self, image: impl Into<String> + Send) -> Result<Self::Results, KonarrError>;

    /// Send the Tool results to the Konarr Server
    async fn send(
        &self,
        client: &KonarrClient,
        snapshot: &KonarrSnapshot,
        results: Self::Results,
    ) -> Result<(), KonarrError>
    where
        Self::Results: Serialize + Send,
    {
        snapshot.upload_bom(client, results).await?;
        Ok(())
    }
}
