// --- server/src/adapters/giphy.rs ---
//
// GIPHY GIF Search adapter. Proxies the official GIPHY REST endpoint so the API
// key stays server-side, and normalizes the provider's verbose JSON into the
// port's small `GifResult`. Search only — no stickers, clips, SDK, or uploads.

use async_trait::async_trait;
use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::ports::{GifResult, GifSearch};

pub struct GiphyClient {
    api_key: Option<String>,
    base_url: String,
    client: reqwest::Client,
}

impl GiphyClient {
    /// `base_url` is the GIPHY API root (e.g. `https://api.giphy.com`); it is a
    /// parameter so tests can point the client at a local fake server.
    pub fn new(api_key: Option<String>, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

// --- GIPHY response subset (only the fields we normalize) ---

#[derive(Deserialize)]
struct GiphyResponse {
    data: Vec<GiphyItem>,
}

#[derive(Deserialize)]
struct GiphyItem {
    id: String,
    #[serde(default)]
    title: String,
    images: GiphyImages,
}

#[derive(Deserialize)]
struct GiphyImages {
    fixed_height: GiphyImage,
    fixed_height_small: Option<GiphyImage>,
}

#[derive(Deserialize)]
struct GiphyImage {
    url: String,
    #[serde(default)]
    width: String,
    #[serde(default)]
    height: String,
}

/// GIPHY reports dimensions as strings; treat anything unexpected as 0.
fn parse_dim(s: &str) -> u32 {
    s.parse().unwrap_or(0)
}

impl From<GiphyItem> for GifResult {
    fn from(item: GiphyItem) -> Self {
        // A small still for the grid; fall back to the display gif if absent.
        let preview_url = item
            .images
            .fixed_height_small
            .as_ref()
            .map(|i| i.url.clone())
            .unwrap_or_else(|| item.images.fixed_height.url.clone());
        GifResult {
            id: item.id,
            title: item.title,
            width: parse_dim(&item.images.fixed_height.width),
            height: parse_dim(&item.images.fixed_height.height),
            url: item.images.fixed_height.url,
            preview_url,
        }
    }
}

#[async_trait]
impl GifSearch for GiphyClient {
    async fn search(
        &self,
        query: &str,
        limit: u32,
        rating: &str,
    ) -> Result<Vec<GifResult>, DomainError> {
        let api_key = self
            .api_key
            .as_deref()
            .ok_or(DomainError::GiphyNotConfigured)?;

        let limit = limit.to_string();
        let response = self
            .client
            .get(format!("{}/v1/gifs/search", self.base_url))
            .query(&[
                ("api_key", api_key),
                ("q", query),
                ("limit", limit.as_str()),
                ("rating", rating),
                ("bundle", "messaging_non_clips"),
            ])
            .send()
            .await
            .map_err(|_| DomainError::ExternalServiceUnavailable)?;

        if !response.status().is_success() {
            return Err(DomainError::ExternalServiceUnavailable);
        }

        let body: GiphyResponse = response
            .json()
            .await
            .map_err(|_| DomainError::ExternalServiceUnavailable)?;

        Ok(body.data.into_iter().map(GifResult::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// A one-shot fake HTTP server returning `status_line` + `body`. Returns its
    /// base URL.
    async fn spawn_server(status_line: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let resp = format!(
                    "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.flush().await;
            }
        });
        format!("http://{addr}")
    }

    const CANNED: &str = r#"{"data":[{"id":"abc","title":"deploy","images":{"fixed_height":{"url":"https://media.giphy.com/media/abc/giphy.gif","width":"200","height":"150"},"fixed_height_small":{"url":"https://media.giphy.com/media/abc/200w_s.gif","width":"100","height":"75"}}}]}"#;

    #[tokio::test]
    async fn a_missing_api_key_is_not_configured() {
        let client = GiphyClient::new(None, "http://127.0.0.1:1".to_string());
        assert_eq!(
            client.search("deploy", 12, "pg").await.unwrap_err(),
            DomainError::GiphyNotConfigured
        );
    }

    #[tokio::test]
    async fn it_normalizes_a_giphy_search_response() {
        let base = spawn_server("HTTP/1.1 200 OK", CANNED).await;
        let client = GiphyClient::new(Some("k".to_string()), base);

        let results = client.search("deploy", 12, "pg").await.unwrap();

        assert_eq!(results.len(), 1);
        let g = &results[0];
        assert_eq!(g.id, "abc");
        assert_eq!(g.title, "deploy");
        assert_eq!(g.url, "https://media.giphy.com/media/abc/giphy.gif");
        assert_eq!(
            g.preview_url,
            "https://media.giphy.com/media/abc/200w_s.gif"
        );
        assert_eq!(g.width, 200);
        assert_eq!(g.height, 150);
    }

    #[tokio::test]
    async fn a_giphy_error_is_external_service_unavailable() {
        let base = spawn_server("HTTP/1.1 500 Internal Server Error", "{}").await;
        let client = GiphyClient::new(Some("k".to_string()), base);

        assert_eq!(
            client.search("deploy", 12, "pg").await.unwrap_err(),
            DomainError::ExternalServiceUnavailable
        );
    }
}
