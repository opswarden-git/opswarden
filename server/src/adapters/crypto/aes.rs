// server/src/adapters/crypto/aes.rs
//
// AES-256-GCM helpers behind the SecretVault. Each encryption draws a fresh
// 96-bit nonce, returned alongside the ciphertext; decryption needs both. The
// GCM authentication tag is carried inside the ciphertext, so any tampering (or a
// wrong key) makes decryption fail rather than return garbage.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};

use crate::domain::error::DomainError;

/// AES-256 key length in bytes.
pub const KEY_LEN: usize = 32;
/// GCM nonce length in bytes.
pub const NONCE_LEN: usize = 12;

/// Encrypt `plaintext`, returning `(nonce, ciphertext)`. The random nonce must be
/// stored next to the ciphertext for later decryption.
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), DomainError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| DomainError::Crypto)?;
    Ok((nonce.as_slice().to_vec(), ciphertext))
}

/// Decrypt a `(nonce, ciphertext)` pair produced by [`encrypt`] under the same key.
pub fn decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, DomainError> {
    if nonce.len() != NONCE_LEN {
        return Err(DomainError::Crypto);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| DomainError::Crypto)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [7u8; KEY_LEN];

    #[test]
    fn round_trip_recovers_the_plaintext() {
        let (nonce, ct) = encrypt(&KEY, b"ghp_super_secret").unwrap();
        let pt = decrypt(&KEY, &nonce, &ct).unwrap();
        assert_eq!(pt, b"ghp_super_secret");
    }

    #[test]
    fn ciphertext_is_not_the_plaintext() {
        let (_, ct) = encrypt(&KEY, b"ghp_super_secret").unwrap();
        assert_ne!(ct.as_slice(), b"ghp_super_secret");
    }

    #[test]
    fn each_encryption_uses_a_fresh_nonce() {
        let (n1, _) = encrypt(&KEY, b"same").unwrap();
        let (n2, _) = encrypt(&KEY, b"same").unwrap();
        assert_ne!(n1, n2);
    }

    #[test]
    fn wrong_key_fails_to_decrypt() {
        let (nonce, ct) = encrypt(&KEY, b"secret").unwrap();
        let other = [9u8; KEY_LEN];
        assert_eq!(decrypt(&other, &nonce, &ct), Err(DomainError::Crypto));
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let (nonce, mut ct) = encrypt(&KEY, b"secret").unwrap();
        ct[0] ^= 0xff;
        assert_eq!(decrypt(&KEY, &nonce, &ct), Err(DomainError::Crypto));
    }
}
