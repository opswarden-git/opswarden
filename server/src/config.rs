// --- server/src/config.rs ---

use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct Config {
    pub kickoff_token_secret: String,
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        let kickoff_token_secret = std::env::var("OPSWARDEN_KICKOFF_TOKEN")
            .unwrap_or_else(|_| "Romeo Cavazza VIGIL2026".to_string());
        
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "my_super_secret_dev_key_12345".to_string());

        Self {
            kickoff_token_secret,
            jwt_secret,
        }
    }

    pub fn kickoff_token(&self) -> String {
        sha256_hex(&self.kickoff_token_secret)
    }
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
