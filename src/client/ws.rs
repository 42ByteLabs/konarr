//! # Konarr Websocket Client

use tokio_tungstenite::tungstenite::connect;
use url::Url;

pub struct KonarrWebsocketClient {
    /// Websocket URL
    url: Url,
}

impl KonarrWebsocketClient {
    pub fn new(url: impl Into<Url>) -> Self {
        let url = url.into();

        Self { url }
    }
}
