use super::{Config, ServerConfig};
use crate::KonarrClient;

impl Config {
    #[cfg(feature = "models")]
    /// Get Database Connection
    pub async fn database(&self) -> Result<geekorm::ConnectionManager, crate::KonarrError> {
        self.database.database().await
    }
}

impl ServerConfig {
    /// Get the Konarr Client
    pub fn client(&self) -> Result<KonarrClient, crate::KonarrError> {
        KonarrClient::init().base(self.api_url()?)?.build()
    }

    /// Get the Konarr Client with Token
    pub fn client_with_token(&self, token: String) -> Result<KonarrClient, crate::KonarrError> {
        KonarrClient::init()
            .base(self.api_url()?)?
            .token(token)
            .build()
    }

    /// Get the Konarr Client with Credentials
    pub fn client_with_credentials(
        &self,
        username: String,
        password: String,
    ) -> Result<KonarrClient, crate::KonarrError> {
        KonarrClient::init()
            .base(self.api_url()?)?
            .credentials(username, password)
            .build()
    }
}
