//! Tools to analyze the BOM of a container image
use crate::{client::snapshot::KonarrSnapshot, KonarrClient, KonarrError};
use async_trait::async_trait;

pub mod grype;
pub mod syft;

pub use grype::Grype;
pub use syft::Syft;

/// Tool Trait
#[async_trait]
pub trait Tool {
    /// Initialize the Tool
    async fn init() -> Result<Self, KonarrError>
    where
        Self: Sized;

    /// Run the Tool
    async fn run(&self, image: impl Into<String> + Send) -> Result<String, KonarrError>;

    /// Send the Tool results to the Konarr Server
    async fn send(
        &self,
        client: &KonarrClient,
        snapshot: &KonarrSnapshot,
        results: String,
    ) -> Result<(), KonarrError> {
        snapshot.upload_bom(client, results).await?;
        Ok(())
    }
}
