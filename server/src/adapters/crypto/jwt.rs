// server/src/adapters/crypto/jwt.rs
use crate::domain::error::DomainError;
use crate::ports::TokenService;
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
        .map_err(|_| DomainError::InvalidCredentials) // Ideally InternalError
    }

    fn verify_token(&self, token: &str) -> Result<uuid::Uuid, DomainError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| DomainError::InvalidToken)?;

        uuid::Uuid::parse_str(&token_data.claims.sub).map_err(|_| DomainError::InvalidToken)
    }
}
