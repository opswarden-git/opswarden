// --- server/src/config.rs ---

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Dev fallback for the AES-256 vault key (32 bytes). Override in any real
/// environment with `OPSWARDEN_VAULT_KEY` (64 hex chars), like `JWT_SECRET`.
const DEV_VAULT_KEY: [u8; 32] = *b"opswarden-dev-vault-key-0123456!";

#[derive(Clone)]
pub struct Config {
    pub kickoff_token_secret: String,
    pub jwt_secret: String,
    /// AES-256-GCM key for the secret vault (see `adapters::pg::vault`).
    pub vault_key: [u8; 32],
    /// Optional GitHub webhook HMAC secret, seeded into the vault at startup.
    pub github_webhook_secret: Option<String>,
    /// Team that automation rules open incidents in (none = rules inert).
    pub automation_team_id: Option<Uuid>,
    /// Outbound URL (Slack incoming webhook, Discord, any HTTP endpoint) for the
    /// Notify REAction; when set alongside a team, a CI failure also notifies it.
    pub automation_notify_url: Option<String>,
    pub google_oauth_client_id: Option<String>,
    pub google_oauth_client_secret: Option<String>,
    pub google_oauth_redirect_uri: String,
    pub web_origin: String,
}

impl Config {
    pub fn from_env() -> Self {
        load_local_env();

        let kickoff_token_secret = std::env::var("OPSWARDEN_KICKOFF_TOKEN")
            .unwrap_or_else(|_| "Romeo Cavazza VIGIL2026".to_string());

        // Fail-fast in release builds: a missing JWT_SECRET in production would
        // silently fall back to a publicly-known key, letting anyone forge tokens.
        // Debug builds keep a dev default for zero-config local work.
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
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

        let vault_key = std::env::var("OPSWARDEN_VAULT_KEY")
            .ok()
            .and_then(|hex_key| decode_key(&hex_key))
            .unwrap_or(DEV_VAULT_KEY);

        let github_webhook_secret = std::env::var("GITHUB_WEBHOOK_SECRET").ok();

        let automation_team_id = std::env::var("OPSWARDEN_AUTOMATION_TEAM_ID")
            .ok()
            .and_then(|raw| Uuid::parse_str(&raw).ok());

        let automation_notify_url = std::env::var("OPSWARDEN_AUTOMATION_NOTIFY_URL").ok();
        let google_oauth_client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID").ok();
        let google_oauth_client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok();
        let google_oauth_redirect_uri = std::env::var("GOOGLE_OAUTH_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8080/api/auth/google/callback".to_string());
        let web_origin = std::env::var("OPSWARDEN_WEB_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:4242".to_string());

        Self {
            kickoff_token_secret,
            jwt_secret,
            vault_key,
            github_webhook_secret,
            automation_team_id,
            automation_notify_url,
            google_oauth_client_id,
            google_oauth_client_secret,
            google_oauth_redirect_uri,
            web_origin,
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
    use super::{is_env_key, sha256_hex};

    #[test]
    fn sha256_is_64_hex_chars_and_deterministic() {
        let digest = sha256_hex("Romeo Cavazza VIGIL2026");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(digest, sha256_hex("Romeo Cavazza VIGIL2026"));
    }

    #[test]
    fn env_keys_require_shell_compatible_names() {
        assert!(is_env_key("GOOGLE_OAUTH_CLIENT_ID"));
        assert!(is_env_key("_PRIVATE"));
        assert!(!is_env_key("1PRIVATE"));
        assert!(!is_env_key("GOOGLE-OAUTH-CLIENT-ID"));
        assert!(!is_env_key(""));
    }
}
