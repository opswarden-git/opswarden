use sha2::{Digest, Sha256};

/// Runtime configuration. At S0 we only need the kickoff token secret.
/// The real string is revealed at the kickoff; until then it is a one-line
/// placeholder swapped via the `OPSWARDEN_KICKOFF_TOKEN` env var.
#[derive(Clone)]
pub struct Config {
    pub kickoff_token_secret: String,
}

impl Config {
    /// Read configuration from the environment, falling back to a documented
    /// placeholder for the kickoff token.
    pub fn from_env() -> Self {
        let kickoff_token_secret = std::env::var("OPSWARDEN_KICKOFF_TOKEN")
            .unwrap_or_else(|_| "Romeo Cavazza VIGIL2026".to_string());
        Self {
            kickoff_token_secret,
        }
    }

    /// SHA-256 hex digest exposed in `/about.json` as required by the subject.
    pub fn kickoff_token(&self) -> String {
        sha256_hex(&self.kickoff_token_secret)
    }
}

/// Lowercase hex SHA-256 of the given input.
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
