// server/src/adapters/crypto/hmac.rs
//
// HMAC-SHA256 webhook signature verification. Implemented directly on `sha2`
// (RFC 2104) rather than pulling the `hmac` crate, which pins an older `digest`
// version than the `sha2` 0.11 already in the tree — fewer deps, no conflict.

use sha2::{Digest, Sha256};

use crate::ports::WebhookVerifier;

const BLOCK_SIZE: usize = 64;

/// HMAC-SHA256(key, message) per RFC 2104.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // Normalize the key to one block: hash it if it's longer, zero-pad otherwise.
    let mut block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        block[..digest.len()].copy_from_slice(&digest);
    } else {
        block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for ((ip, op), kb) in ipad.iter_mut().zip(opad.iter_mut()).zip(block.iter()) {
        *ip ^= *kb;
        *op ^= *kb;
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);

    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

/// Constant-time byte comparison: no early return on the first mismatch, so the
/// time taken doesn't leak how much of the signature was correct.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// GitHub-style HMAC-SHA256 verifier. The signature header is
/// `X-Hub-Signature-256: sha256=<hex>`; a bare hex digest is also accepted.
pub struct HmacSha256Verifier;

impl WebhookVerifier for HmacSha256Verifier {
    fn verify(&self, secret: &str, body: &[u8], signature: &str) -> bool {
        let provided = signature.strip_prefix("sha256=").unwrap_or(signature);
        let Ok(provided_bytes) = hex::decode(provided) else {
            return false;
        };
        let expected = hmac_sha256(secret.as_bytes(), body);
        constant_time_eq(&expected, &provided_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4231 test case 1: key = 0x0b*20, data = "Hi There".
    #[test]
    fn matches_rfc4231_vector_1() {
        let key = [0x0bu8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex::encode(mac),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn verifier_accepts_a_valid_github_signature() {
        let secret = "it's a secret";
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        let sig = format!(
            "sha256={}",
            hex::encode(hmac_sha256(secret.as_bytes(), body))
        );
        assert!(HmacSha256Verifier.verify(secret, body, &sig));
    }

    #[test]
    fn verifier_rejects_a_tampered_body() {
        let secret = "it's a secret";
        let body = br#"{"workflow_run":{"conclusion":"failure"}}"#;
        let sig = format!(
            "sha256={}",
            hex::encode(hmac_sha256(secret.as_bytes(), body))
        );
        assert!(!HmacSha256Verifier.verify(secret, b"tampered", &sig));
    }

    #[test]
    fn verifier_rejects_a_wrong_secret_and_garbage() {
        let body = b"payload";
        let sig = format!("sha256={}", hex::encode(hmac_sha256(b"right", body)));
        assert!(!HmacSha256Verifier.verify("wrong", body, &sig));
        assert!(!HmacSha256Verifier.verify("right", body, "sha256=not-hex"));
        assert!(!HmacSha256Verifier.verify("right", body, ""));
    }
}
