// --- server/src/config.rs ---

use sha2::{Digest, Sha256};

/// Dev fallback for the AES-256 vault key (32 bytes). Override in any real
/// environment with `OPSWARDEN_VAULT_KEY` (64 hex chars), like `JWT_SECRET`.
const DEV_VAULT_KEY: [u8; 32] = *b"opswarden-dev-vault-key-0123456!";

#[derive(Clone)]
pub struct Config {
    pub kickoff_token_secret: String,
    pub jwt_secret: String,
    /// AES-256-GCM key for Team-owned connection credentials.
    pub vault_key: [u8; 32],
    pub google_oauth_client_id: Option<String>,
    pub google_oauth_client_secret: Option<String>,
    pub google_oauth_redirect_uri: String,
    /// Dedicated GitHub identity OAuth application. Kept separate from the
    /// Team automation integration, whose tokens carry repository access.
    pub github_auth_client_id: Option<String>,
    pub github_auth_client_secret: Option<String>,
    pub github_auth_redirect_uri: String,
    pub github_oauth_client_id: Option<String>,
    pub github_oauth_client_secret: Option<String>,
    pub github_oauth_redirect_uri: String,
    pub web_origin: String,
    /// Exact browser origins allowed to open a WebSocket handshake. Requests
    /// without Origin remain valid for native and service clients.
    pub ws_allowed_origins: Vec<String>,
    /// Number of reverse-proxy hops controlled by the deployment. `0` means
    /// forwarded client-address headers are ignored.
    pub trusted_proxy_hops: usize,
    /// GIPHY REST API key for timeline GIF search (server-side only — never
    /// exposed to the client). `None` => the search endpoint reports
    /// `giphy_not_configured`.
    pub giphy_api_key: Option<String>,
    /// Poll cadence for the durable Timer worker. Claims remain coordinated by
    /// PostgreSQL; this only controls how quickly a replica looks for work.
    pub timer_poll_seconds: u64,
    /// Socket the HTTP server listens on. Configurable so a native `just dev`
    /// run can coexist with the Compose stack, which already publishes 8080.
    pub bind_addr: String,
    /// Coarse ceiling on unauthenticated `/api/auth/*` attempts per client
    /// address. Loose on purpose: a proxy that forwards no client address puts
    /// every visitor in one bucket. Counted per replica.
    pub auth_rate_limit_attempts: u32,
    /// Sign-in attempts allowed per account. This is the limit that bounds
    /// credential stuffing, and it is unaffected by proxy topology.
    pub auth_rate_limit_per_account: u32,
    pub auth_rate_limit_window_seconds: u64,
}

impl Config {
    pub fn from_env() -> Self {
        load_local_env();

        // Every optional var goes through `optional_env`, which treats a blank or
        // whitespace-only value as unset. This matters for the compose demo path:
        // `${VAR:-}` passes an empty string when the host hasn't set it, and an
        // empty HMAC secret / OAuth id / notify URL must mean "not configured",
        // never a meaningless `Some("")`.
        let kickoff_token_secret =
            optional_env("OPSWARDEN_KICKOFF_TOKEN").unwrap_or_else(|| "OpsWarden".to_string());

        // Fail-fast in release builds: a missing (or blank) JWT_SECRET in
        // production would silently fall back to a publicly-known key, letting
        // anyone forge tokens. Debug builds keep a dev default for zero-config work.
        let jwt_secret = optional_env("JWT_SECRET").unwrap_or_else(|| {
            if cfg!(debug_assertions) {
                eprintln!(
                    "WARNING: JWT_SECRET is unset — using an insecure development default \
                     (allowed in debug builds only)."
                );
                "my_super_secret_dev_key_12345".to_string()
            } else {
                panic!(
                    "JWT_SECRET must be set: refusing to start a release build with a public \
                     default signing key."
                );
            }
        });

        // A blank OPSWARDEN_VAULT_KEY falls back to the dev key (unchanged behavior).
        let vault_key = optional_env("OPSWARDEN_VAULT_KEY")
            .and_then(|hex_key| decode_key(&hex_key))
            .unwrap_or(DEV_VAULT_KEY);

        let google_oauth_client_id = optional_env("GOOGLE_OAUTH_CLIENT_ID");
        let google_oauth_client_secret = optional_env("GOOGLE_OAUTH_CLIENT_SECRET");
        let google_oauth_redirect_uri = optional_env("GOOGLE_OAUTH_REDIRECT_URI")
            .unwrap_or_else(|| "http://localhost:8080/api/auth/google/callback".to_string());
        let github_auth_client_id = optional_env("GITHUB_AUTH_CLIENT_ID");
        let github_auth_client_secret = optional_env("GITHUB_AUTH_CLIENT_SECRET");
        let github_auth_redirect_uri = optional_env("GITHUB_AUTH_REDIRECT_URI")
            .unwrap_or_else(|| "http://localhost:8080/api/auth/github/callback".to_string());
        let github_oauth_client_id = optional_env("GITHUB_OAUTH_CLIENT_ID");
        let github_oauth_client_secret = optional_env("GITHUB_OAUTH_CLIENT_SECRET");
        let github_oauth_redirect_uri =
            optional_env("GITHUB_OAUTH_REDIRECT_URI").unwrap_or_else(|| {
                "http://localhost:8080/api/service-oauth/github/callback".to_string()
            });
        let web_origin = optional_env("OPSWARDEN_WEB_ORIGIN")
            .unwrap_or_else(|| "http://localhost:4242".to_string());
        let ws_allowed_origins = optional_env("OPSWARDEN_WS_ALLOWED_ORIGINS")
            .map(|value| parse_origins(&value))
            .filter(|origins| !origins.is_empty())
            .unwrap_or_else(|| vec![web_origin.trim_end_matches('/').to_string()]);
        let trusted_proxy_hops = optional_env("OPSWARDEN_TRUSTED_PROXY_HOPS")
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|hops| *hops <= 16)
            .unwrap_or(0);
        let giphy_api_key = optional_env("GIPHY_API_KEY");
        let timer_poll_seconds = optional_env("OPSWARDEN_TIMER_POLL_SECONDS")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|seconds| (5..=60).contains(seconds))
            .unwrap_or(15);
        // Unchanged default: every deployment that sets nothing keeps 0.0.0.0:8080.
        let bind_addr =
            optional_env("OPSWARDEN_BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".to_string());
        // Loose: this bucket may hold an entire deployment when the proxy in
        // front forwards no client address.
        let auth_rate_limit_attempts = optional_env("OPSWARDEN_AUTH_RATE_LIMIT_ATTEMPTS")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|attempts| (1..=100_000).contains(attempts))
            .unwrap_or(600);
        // Tight: 10 tries per 5 minutes on one account leaves a forgetful human
        // comfortable and turns guessing into two tries a minute.
        let auth_rate_limit_per_account = optional_env("OPSWARDEN_AUTH_RATE_LIMIT_PER_ACCOUNT")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|attempts| (1..=1000).contains(attempts))
            .unwrap_or(10);
        let auth_rate_limit_window_seconds =
            optional_env("OPSWARDEN_AUTH_RATE_LIMIT_WINDOW_SECONDS")
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|seconds| (1..=3600).contains(seconds))
                .unwrap_or(300);

        Self {
            kickoff_token_secret,
            jwt_secret,
            vault_key,
            google_oauth_client_id,
            google_oauth_client_secret,
            google_oauth_redirect_uri,
            github_auth_client_id,
            github_auth_client_secret,
            github_auth_redirect_uri,
            github_oauth_client_id,
            github_oauth_client_secret,
            github_oauth_redirect_uri,
            web_origin,
            ws_allowed_origins,
            trusted_proxy_hops,
            giphy_api_key,
            timer_poll_seconds,
            bind_addr,
            auth_rate_limit_attempts,
            auth_rate_limit_per_account,
            auth_rate_limit_window_seconds,
        }
    }

    pub fn kickoff_token(&self) -> String {
        sha256_hex(&self.kickoff_token_secret)
    }
}

/// Decode a 64-hex-char string into a 32-byte AES key, or `None` if malformed.
fn decode_key(hex_key: &str) -> Option<[u8; 32]> {
    hex::decode(hex_key).ok()?.try_into().ok()
}

/// `None` if the value is absent, empty, or whitespace-only; otherwise the value
/// returned **unchanged** (secrets are preserved exactly, never trimmed). Keeps a
/// blank env var — common when a compose `${VAR:-}` is left unset — from becoming
/// a meaningless `Some("")` such as an empty HMAC secret or `""` OAuth client id.
fn nonblank(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// Read an optional environment variable, treating blank/whitespace as unset.
fn optional_env(key: &str) -> Option<String> {
    nonblank(std::env::var(key).ok())
}

fn parse_origins(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .map(|origin| origin.trim_end_matches('/'))
        .filter(|origin| origin.starts_with("https://") || origin.starts_with("http://"))
        .map(str::to_string)
        .collect()
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn load_local_env() {
    for path in [".env", "server/.env"] {
        if let Ok(contents) = std::fs::read_to_string(path) {
            load_env_contents(&contents);
        }
    }
}

fn load_env_contents(contents: &str) {
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !is_env_key(key) || std::env::var_os(key).is_some() {
            continue;
        }

        let value = raw_value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|inner| inner.strip_suffix('\''))
            })
            .unwrap_or(value);

        std::env::set_var(key, value);
    }
}

fn is_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::{is_env_key, nonblank, parse_origins, sha256_hex};

    #[test]
    fn nonblank_treats_empty_and_whitespace_as_none() {
        assert_eq!(nonblank(None), None);
        assert_eq!(nonblank(Some(String::new())), None);
        assert_eq!(nonblank(Some("   ".to_string())), None);
        assert_eq!(nonblank(Some("\n\t ".to_string())), None);
        // A real value is kept exactly, never trimmed (secrets stay intact).
        assert_eq!(
            nonblank(Some("hmac-secret".to_string())).as_deref(),
            Some("hmac-secret")
        );
        assert_eq!(
            nonblank(Some("  pad  ".to_string())).as_deref(),
            Some("  pad  ")
        );
    }

    #[test]
    fn sha256_is_64_hex_chars_and_deterministic() {
        let digest = sha256_hex("OpsWarden");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(digest, sha256_hex("OpsWarden"));
    }

    #[test]
    fn env_keys_require_shell_compatible_names() {
        assert!(is_env_key("GOOGLE_OAUTH_CLIENT_ID"));
        assert!(is_env_key("_PRIVATE"));
        assert!(!is_env_key("1PRIVATE"));
        assert!(!is_env_key("GOOGLE-OAUTH-CLIENT-ID"));
        assert!(!is_env_key(""));
    }

    #[test]
    fn websocket_origins_are_exact_normalized_http_origins() {
        assert_eq!(
            parse_origins(" https://app.opswarden.dev/,http://localhost:4242, null "),
            vec![
                "https://app.opswarden.dev".to_string(),
                "http://localhost:4242".to_string()
            ]
        );
    }
}
