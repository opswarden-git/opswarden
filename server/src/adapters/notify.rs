// --- server/src/adapters/notify.rs ---
//
// Outbound HTTP notifier (the `Notify` REAction). POSTs `{"text": message}` to
// the target URL — that is the Slack incoming-webhook contract, which also works
// for Discord, Teams, and any endpoint accepting a JSON body. One connector,
// many targets: the rule just carries a `url`.

use async_trait::async_trait;
use serde_json::json;

use crate::domain::error::DomainError;
use crate::ports::Notifier;

pub struct HttpNotifier {
    client: reqwest::Client,
}

impl HttpNotifier {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError> {
        let response = self
            .client
            .post(url)
            .json(&json!({ "text": message }))
            .send()
            .await
            .map_err(|_| DomainError::ReactionFailed)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(DomainError::ReactionFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Spawn a throwaway one-shot HTTP server that captures the request and
    /// replies with `status_line`. Returns the POST URL and a receiver that
    /// yields the raw request once it arrives.
    async fn spawn_server(
        status_line: &'static str,
    ) -> (String, tokio::sync::oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 2048];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let resp =
                    format!("{status_line}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = tx.send(req);
            }
        });
        (format!("http://{addr}/hook"), rx)
    }

    #[tokio::test]
    async fn posts_the_text_payload_and_succeeds_on_2xx() {
        let (url, rx) = spawn_server("HTTP/1.1 200 OK").await;

        HttpNotifier::new()
            .notify(&url, "incident escalated")
            .await
            .unwrap();

        let req = rx.await.unwrap();
        assert!(req.starts_with("POST /hook"));
        assert!(req.contains(r#""text":"incident escalated""#));
    }

    #[tokio::test]
    async fn a_non_2xx_response_is_a_reaction_failure() {
        let (url, _rx) = spawn_server("HTTP/1.1 500 Internal Server Error").await;

        assert_eq!(
            HttpNotifier::new().notify(&url, "boom").await.unwrap_err(),
            DomainError::ReactionFailed
        );
    }

    #[tokio::test]
    async fn an_unreachable_url_is_a_reaction_failure() {
        // Bind then drop to obtain a port that is now definitely closed.
        let addr = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };

        assert_eq!(
            HttpNotifier::new()
                .notify(&format!("http://{addr}/nope"), "x")
                .await
                .unwrap_err(),
            DomainError::ReactionFailed
        );
    }
}
