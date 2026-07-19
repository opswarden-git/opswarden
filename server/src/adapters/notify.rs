//! Bounded outbound HTTP notification adapter.
//!
//! Endpoint URLs are credentials because webhook paths often contain tokens.
//! The application passes the decrypted value only to this adapter. Every send
//! parses and resolves the destination again, rejects non-public addresses, and
//! pins the validated addresses into reqwest to avoid a second DNS decision.

use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;
use std::time::Duration;

use async_trait::async_trait;
use ipnet::IpNet;
use reqwest::{redirect::Policy, Url};
use serde_json::json;

use crate::domain::error::DomainError;
use crate::ports::Notifier;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_TOTAL_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;
const DEFAULT_MAX_PAYLOAD_BYTES: usize = 8 * 1024;

const FORBIDDEN_NETWORKS: &[&str] = &[
    // IPv4 special-use, local and non-unicast networks.
    "0.0.0.0/8",
    "10.0.0.0/8",
    "100.64.0.0/10",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "172.16.0.0/12",
    "192.0.0.0/24",
    "192.0.2.0/24",
    "192.88.99.0/24",
    "192.168.0.0/16",
    "198.18.0.0/15",
    "198.51.100.0/24",
    "203.0.113.0/24",
    "224.0.0.0/4",
    "240.0.0.0/4",
    // IPv6 unspecified, local, translation, documentation and transition ranges.
    "::/128",
    "::1/128",
    "::ffff:0:0/96",
    "64:ff9b::/96",
    "64:ff9b:1::/48",
    "100::/64",
    "2001::/23",
    "2001:db8::/32",
    "2002::/16",
    "3fff::/20",
    "5f00::/16",
    "fc00::/7",
    "fe80::/10",
    "fec0::/10",
    "ff00::/8",
];

static FORBIDDEN_IP_NETS: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    FORBIDDEN_NETWORKS
        .iter()
        .map(|network| network.parse().expect("valid static IP network"))
        .collect()
});

pub struct HttpNotifier {
    connect_timeout: Duration,
    total_timeout: Duration,
    max_response_bytes: usize,
    max_payload_bytes: usize,
    allow_test_targets: bool,
}

struct ResolvedTarget {
    url: Url,
    host: String,
    addresses: Vec<SocketAddr>,
}

impl HttpNotifier {
    pub fn new() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            total_timeout: DEFAULT_TOTAL_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            allow_test_targets: false,
        }
    }

    async fn resolve_target(&self, raw_url: &str) -> Result<ResolvedTarget, DomainError> {
        let url = parse_endpoint(raw_url, self.allow_test_targets)?;
        let host = url
            .host_str()
            .ok_or(DomainError::InvalidReactionEndpoint)?
            .to_ascii_lowercase();
        let port = url
            .port_or_known_default()
            .ok_or(DomainError::InvalidReactionEndpoint)?;

        let mut addresses = if self.allow_test_targets {
            match literal_ip(&host) {
                Ok(ip) => vec![SocketAddr::new(ip, port)],
                Err(_) => self.resolve_dns_bounded(&host, port).await?,
            }
        } else {
            self.resolve_dns_bounded(&host, port).await?
        };
        addresses.sort_unstable();
        addresses.dedup();
        if addresses.is_empty() {
            return Err(DomainError::ReactionNetworkError);
        }
        if !self.allow_test_targets {
            validate_public_addresses(&addresses)?;
        }

        Ok(ResolvedTarget {
            url,
            host,
            addresses,
        })
    }

    async fn resolve_dns_bounded(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, DomainError> {
        tokio::time::timeout(self.connect_timeout, resolve_dns(host, port))
            .await
            .map_err(|_| DomainError::ReactionTimeout)?
    }

    async fn send_to_target(
        &self,
        target: ResolvedTarget,
        message: &str,
    ) -> Result<(), DomainError> {
        let payload = serde_json::to_vec(&json!({ "text": message }))
            .map_err(|_| DomainError::ReactionPayloadTooLarge)?;
        if payload.len() > self.max_payload_bytes {
            return Err(DomainError::ReactionPayloadTooLarge);
        }

        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(self.connect_timeout)
            .timeout(self.total_timeout)
            .resolve_to_addrs(&target.host, &target.addresses)
            .build()
            .map_err(|_| DomainError::ReactionNetworkError)?;
        let mut response = client
            .post(target.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(payload)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        match response.status() {
            status if status.is_success() => {}
            status if status.is_redirection() => {
                return Err(DomainError::ReactionRedirectRefused);
            }
            status if status.is_client_error() => return Err(DomainError::ReactionHttp4xx),
            status if status.is_server_error() => return Err(DomainError::ReactionHttp5xx),
            _ => return Err(DomainError::ReactionFailed),
        }

        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(DomainError::ReactionResponseTooLarge);
        }

        let mut received = 0usize;
        while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
            received = received.saturating_add(chunk.len());
            if received > self.max_response_bytes {
                return Err(DomainError::ReactionResponseTooLarge);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn for_tests(total_timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            connect_timeout: total_timeout,
            total_timeout,
            max_response_bytes,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            allow_test_targets: true,
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
    async fn validate_endpoint(&self, url: &str) -> Result<(), DomainError> {
        self.resolve_target(url).await.map(|_| ())
    }

    async fn notify(&self, url: &str, message: &str) -> Result<(), DomainError> {
        let target = self.resolve_target(url).await?;
        self.send_to_target(target, message).await
    }
}

fn parse_endpoint(raw_url: &str, allow_test_targets: bool) -> Result<Url, DomainError> {
    let url = Url::parse(raw_url.trim()).map_err(|_| DomainError::InvalidReactionEndpoint)?;
    let host = url.host_str().ok_or(DomainError::InvalidReactionEndpoint)?;
    let host = host.to_ascii_lowercase();
    let private_name = host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".home.arpa")
        || host == "metadata.amazonaws.com";
    let is_literal_ip = literal_ip(&host).is_ok();
    let scheme_allowed = url.scheme() == "https" || (allow_test_targets && url.scheme() == "http");
    let port_allowed = url.port_or_known_default() == Some(443) || allow_test_targets;
    if !scheme_allowed
        || !port_allowed
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || (!allow_test_targets && (private_name || is_literal_ip))
    {
        return Err(DomainError::InvalidReactionEndpoint);
    }
    Ok(url)
}

async fn resolve_dns(host: &str, port: u16) -> Result<Vec<SocketAddr>, DomainError> {
    tokio::net::lookup_host((host, port))
        .await
        .map(|addresses| addresses.collect())
        .map_err(|_| DomainError::ReactionNetworkError)
}

fn is_public_ip(ip: IpAddr) -> bool {
    !FORBIDDEN_IP_NETS
        .iter()
        .any(|network| network.contains(&ip))
}

fn validate_public_addresses(addresses: &[SocketAddr]) -> Result<(), DomainError> {
    if addresses.iter().all(|address| is_public_ip(address.ip())) {
        Ok(())
    } else {
        Err(DomainError::UnsafeReactionTarget)
    }
}

fn literal_ip(host: &str) -> Result<IpAddr, std::net::AddrParseError> {
    host.trim_start_matches('[').trim_end_matches(']').parse()
}

fn map_reqwest_error(error: reqwest::Error) -> DomainError {
    if error.is_timeout() {
        DomainError::ReactionTimeout
    } else {
        DomainError::ReactionNetworkError
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_server(
        status: StatusCode,
        body: Vec<u8>,
        delay: Duration,
    ) -> (String, tokio::sync::oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut request = vec![0u8; 4096];
                let size = socket.read(&mut request).await.unwrap_or(0);
                tokio::time::sleep(delay).await;
                let reason = status.canonical_reason().unwrap_or("Test");
                let head = format!(
                    "HTTP/1.1 {} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    status.as_u16(),
                    body.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&body).await;
                let _ = socket.flush().await;
                let _ = tx.send(String::from_utf8_lossy(&request[..size]).to_string());
            }
        });
        (format!("http://{address}/hook"), rx)
    }

    #[test]
    fn endpoint_contract_is_https_dns_and_credential_free() {
        for invalid in [
            "http://example.com/hook",
            "https://user:secret@example.com/hook",
            "https://example.com:8443/hook",
            "https://example.com/hook#fragment",
            "https://localhost/hook",
            "https://127.0.0.1/hook",
            "file:///etc/passwd",
        ] {
            assert_eq!(
                parse_endpoint(invalid, false).unwrap_err(),
                DomainError::InvalidReactionEndpoint,
                "{invalid}"
            );
        }
        assert!(parse_endpoint("https://hooks.example.com/a?token=secret", false).is_ok());
    }

    #[test]
    fn special_use_ipv4_and_ipv6_are_not_public() {
        for blocked in [
            "0.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "192.168.0.1",
            "198.18.0.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!is_public_ip(blocked.parse().unwrap()), "{blocked}");
        }
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));

        let mixed = [
            "8.8.8.8:443".parse().unwrap(),
            "127.0.0.1:443".parse().unwrap(),
        ];
        assert_eq!(
            validate_public_addresses(&mixed).unwrap_err(),
            DomainError::UnsafeReactionTarget
        );
    }

    #[tokio::test]
    async fn posts_a_bounded_json_payload_to_an_already_validated_target() {
        let (url, request) = spawn_server(StatusCode::OK, vec![], Duration::ZERO).await;
        let notifier = HttpNotifier::for_tests(Duration::from_secs(1), 1024);
        notifier.notify(&url, "incident escalated").await.unwrap();
        let request = request.await.unwrap();
        assert!(request.starts_with("POST /hook"));
        assert!(request.contains(r#""text":"incident escalated""#));
    }

    #[tokio::test]
    async fn refuses_redirects_without_following_them() {
        let (url, _request) = spawn_server(StatusCode::FOUND, vec![], Duration::ZERO).await;
        let notifier = HttpNotifier::for_tests(Duration::from_secs(1), 1024);
        assert_eq!(
            notifier.notify(&url, "test").await.unwrap_err(),
            DomainError::ReactionRedirectRefused
        );
    }

    #[tokio::test]
    async fn bounds_timeout_and_response_body() {
        let (slow_url, _request) =
            spawn_server(StatusCode::OK, vec![], Duration::from_millis(100)).await;
        let impatient = HttpNotifier::for_tests(Duration::from_millis(20), 1024);
        assert_eq!(
            impatient.notify(&slow_url, "test").await.unwrap_err(),
            DomainError::ReactionTimeout
        );

        let (large_url, _request) =
            spawn_server(StatusCode::OK, vec![b'x'; 33], Duration::ZERO).await;
        let bounded = HttpNotifier::for_tests(Duration::from_secs(1), 32);
        assert_eq!(
            bounded.notify(&large_url, "test").await.unwrap_err(),
            DomainError::ReactionResponseTooLarge
        );
    }

    #[tokio::test]
    async fn maps_remote_statuses_without_reading_or_exposing_their_body() {
        for (status, expected) in [
            (StatusCode::BAD_REQUEST, DomainError::ReactionHttp4xx),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                DomainError::ReactionHttp5xx,
            ),
        ] {
            let (url, _request) =
                spawn_server(status, b"remote secret".to_vec(), Duration::ZERO).await;
            let notifier = HttpNotifier::for_tests(Duration::from_secs(1), 1024);
            assert_eq!(notifier.notify(&url, "test").await.unwrap_err(), expected);
        }
    }
}
