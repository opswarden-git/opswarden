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
}

impl Config {
    pub fn from_env() -> Self {
        let kickoff_token_secret = std::env::var("OPSWARDEN_KICKOFF_TOKEN")
            .unwrap_or_else(|_| "Romeo Cavazza VIGIL2026".to_string());

        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "my_super_secret_dev_key_12345".to_string());

        let vault_key = std::env::var("OPSWARDEN_VAULT_KEY")
            .ok()
            .and_then(|hex_key| decode_key(&hex_key))
            .unwrap_or(DEV_VAULT_KEY);

        let github_webhook_secret = std::env::var("GITHUB_WEBHOOK_SECRET").ok();

        let automation_team_id = std::env::var("OPSWARDEN_AUTOMATION_TEAM_ID")
            .ok()
            .and_then(|raw| Uuid::parse_str(&raw).ok());

        let automation_notify_url = std::env::var("OPSWARDEN_AUTOMATION_NOTIFY_URL").ok();

        Self {
            kickoff_token_secret,
            jwt_secret,
            vault_key,
            github_webhook_secret,
            automation_team_id,
            automation_notify_url,
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

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_is_64_hex_chars_and_deterministic() {
        let digest = sha256_hex("Romeo Cavazza VIGIL2026");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(digest, sha256_hex("Romeo Cavazza VIGIL2026"));
    }
}
