// server/src/adapters/crypto/aes.rs
//
// AES-256-GCM helpers behind the Team connection credential vault. Each encryption draws a fresh
// 96-bit nonce, returned alongside the ciphertext; decryption needs both. The
// GCM authentication tag is carried inside the ciphertext, so any tampering (or a
// wrong key, or mismatched AAD) makes decryption fail rather than return garbage.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Key, Nonce,
};
use uuid::Uuid;

use crate::domain::error::DomainError;

/// AES-256 key length in bytes.
pub const KEY_LEN: usize = 32;
/// GCM nonce length in bytes.
pub const NONCE_LEN: usize = 12;

/// Default vault key version.
pub const DEFAULT_KEY_VERSION: u8 = 1;

/// Constructs a canonical Additional Authenticated Data (AAD) byte vector binding
/// a credential secret to its target connection, credential category, and key version.
pub fn canonical_aad(connection_id: &Uuid, credential_kind: &str, key_version: u8) -> Vec<u8> {
    format!("v{key_version}:{connection_id}:{credential_kind}").into_bytes()
}

/// Encrypt `plaintext` with `aad`, returning `(nonce, ciphertext)`. The random nonce must be
/// stored next to the ciphertext for later decryption.
pub fn encrypt(
    key: &[u8; KEY_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), DomainError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let payload = Payload {
        msg: plaintext,
        aad,
    };
    let ciphertext = cipher
        .encrypt(&nonce, payload)
        .map_err(|_| DomainError::Crypto)?;
    Ok((nonce.as_slice().to_vec(), ciphertext))
}

/// Decrypt a `(nonce, ciphertext)` pair produced by [`encrypt`] under the same key.
/// If decryption with `aad` fails, attempts a fallback with empty AAD for legacy rows.
pub fn decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, DomainError> {
    if nonce.len() != NONCE_LEN {
        return Err(DomainError::Crypto);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let payload = Payload {
        msg: ciphertext,
        aad,
    };
    cipher
        .decrypt(Nonce::from_slice(nonce), payload)
        .or_else(|_| cipher.decrypt(Nonce::from_slice(nonce), ciphertext))
        .map_err(|_| DomainError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [7u8; KEY_LEN];

    #[test]
    fn canonical_aad_format_is_deterministic() {
        let conn_id = Uuid::nil();
        let aad = canonical_aad(&conn_id, "token", 1);
        assert_eq!(
            String::from_utf8(aad).unwrap(),
            "v1:00000000-0000-0000-0000-000000000000:token"
        );
    }

    #[test]
    fn round_trip_recovers_the_plaintext_with_aad() {
        let conn_id = Uuid::new_v4();
        let aad = canonical_aad(&conn_id, "api_key", DEFAULT_KEY_VERSION);
        let (nonce, ct) = encrypt(&KEY, b"ghp_super_secret", &aad).unwrap();
        let pt = decrypt(&KEY, &nonce, &ct, &aad).unwrap();
        assert_eq!(pt, b"ghp_super_secret");
    }

    #[test]
    fn ciphertext_is_not_the_plaintext() {
        let conn_id = Uuid::new_v4();
        let aad = canonical_aad(&conn_id, "api_key", DEFAULT_KEY_VERSION);
        let (_, ct) = encrypt(&KEY, b"ghp_super_secret", &aad).unwrap();
        assert_ne!(ct.as_slice(), b"ghp_super_secret");
    }

    #[test]
    fn each_encryption_uses_a_fresh_nonce() {
        let conn_id = Uuid::new_v4();
        let aad = canonical_aad(&conn_id, "api_key", DEFAULT_KEY_VERSION);
        let (n1, _) = encrypt(&KEY, b"same", &aad).unwrap();
        let (n2, _) = encrypt(&KEY, b"same", &aad).unwrap();
        assert_ne!(n1, n2);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let conn_id = Uuid::new_v4();
        let aad = canonical_aad(&conn_id, "api_key", DEFAULT_KEY_VERSION);
        let (nonce, ct) = encrypt(&KEY, b"secret", &aad).unwrap();
        let other = [9u8; KEY_LEN];
        assert_eq!(decrypt(&other, &nonce, &ct, &aad), Err(DomainError::Crypto));
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let conn_id = Uuid::new_v4();
        let aad = canonical_aad(&conn_id, "api_key", DEFAULT_KEY_VERSION);
        let (nonce, mut ct) = encrypt(&KEY, b"secret", &aad).unwrap();
        ct[0] ^= 0xff;
        assert_eq!(decrypt(&KEY, &nonce, &ct, &aad), Err(DomainError::Crypto));
    }

    #[test]
    fn mismatched_aad_fails_decryption() {
        let conn1 = Uuid::new_v4();
        let conn2 = Uuid::new_v4();
        let aad1 = canonical_aad(&conn1, "api_key", DEFAULT_KEY_VERSION);
        let aad2 = canonical_aad(&conn2, "api_key", DEFAULT_KEY_VERSION);
        let (nonce, ct) = encrypt(&KEY, b"secret", &aad1).unwrap();
        // Trying to decrypt a secret belonging to conn1 under conn2's AAD fails
        assert_eq!(decrypt(&KEY, &nonce, &ct, &aad2), Err(DomainError::Crypto));
    }

    #[test]
    fn legacy_empty_aad_fallback_succeeds() {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&KEY));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = cipher.encrypt(&nonce, b"legacy_secret".as_slice()).unwrap();
        let aad = canonical_aad(&Uuid::new_v4(), "api_key", DEFAULT_KEY_VERSION);
        let pt = decrypt(&KEY, nonce.as_slice(), &ct, &aad).unwrap();
        assert_eq!(pt, b"legacy_secret");
    }
}
