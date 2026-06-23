// server/src/adapters/crypto/jwt.rs
use crate::domain::error::DomainError;
use crate::ports::{TokenClaims, TokenService};
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub struct JwtTokenService {
    secret: String,
}

impl JwtTokenService {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl TokenService for JwtTokenService {
    fn generate_token(&self, user_id: uuid::Uuid) -> Result<String, DomainError> {
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|_| DomainError::Storage)
    }

    fn verify_token(&self, token: &str) -> Result<TokenClaims, DomainError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| DomainError::InvalidToken)?;

        let user_id =
            uuid::Uuid::parse_str(&token_data.claims.sub).map_err(|_| DomainError::InvalidToken)?;
        let expires_at = DateTime::<Utc>::from_timestamp(token_data.claims.exp as i64, 0)
            .ok_or(DomainError::InvalidToken)?;

        Ok(TokenClaims {
            user_id,
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn roundtrip_preserves_user_id_and_sets_future_expiry() {
        let svc = JwtTokenService::new("test-secret".to_string());
        let uid = Uuid::new_v4();

        let token = svc.generate_token(uid).unwrap();
        let claims = svc.verify_token(&token).unwrap();

        assert_eq!(claims.user_id, uid);
        assert!(claims.expires_at > Utc::now());
    }

    #[test]
    fn a_token_signed_with_another_secret_is_rejected() {
        let signer = JwtTokenService::new("secret-a".to_string());
        let verifier = JwtTokenService::new("secret-b".to_string());

        let token = signer.generate_token(Uuid::new_v4()).unwrap();

        assert_eq!(
            verifier.verify_token(&token).unwrap_err(),
            DomainError::InvalidToken
        );
    }

    #[test]
    fn a_garbage_or_tampered_token_is_rejected() {
        let svc = JwtTokenService::new("test-secret".to_string());

        assert_eq!(
            svc.verify_token("not.a.jwt").unwrap_err(),
            DomainError::InvalidToken
        );

        let mut tampered = svc.generate_token(Uuid::new_v4()).unwrap();
        tampered.push('x'); // corrupt the signature segment
        assert_eq!(
            svc.verify_token(&tampered).unwrap_err(),
            DomainError::InvalidToken
        );
    }
}
