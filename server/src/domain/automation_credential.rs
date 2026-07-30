use std::fmt;

use super::error::DomainError;

/// The purpose of one encrypted value attached to a service connection.
/// Providers may need more than one kind (for example a PAT plus a webhook
/// signing secret), so this is a row discriminator rather than a column name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CredentialKind {
    WebhookSigningSecret,
    PersonalToken,
    OAuthAccessToken,
    OAuthRefreshToken,
    EndpointUrl,
    SmtpHost,
    SmtpPort,
    SmtpUsername,
    SmtpPassword,
    FromAddress,
}

impl CredentialKind {
    /// Every variant, so a storage test can prove the whole enum survives a
    /// round-trip through Postgres. `service_connection_secrets.kind` carries an
    /// allowlist constraint, and the Email vertical shipped five variants that
    /// were missing from it; the in-memory vault used by the HTTP tests enforces
    /// nothing, so nothing failed until production.
    ///
    /// `exhaustiveness_guard` below makes the compiler reject a new variant that
    /// is not added here.
    pub const ALL: &'static [CredentialKind] = &[
        Self::WebhookSigningSecret,
        Self::PersonalToken,
        Self::OAuthAccessToken,
        Self::OAuthRefreshToken,
        Self::EndpointUrl,
        Self::SmtpHost,
        Self::SmtpPort,
        Self::SmtpUsername,
        Self::SmtpPassword,
        Self::FromAddress,
    ];

    /// Not called at runtime. The match is exhaustive on purpose: adding a
    /// variant stops compilation here, which is the reminder to extend `ALL` and
    /// to write the migration that widens the storage allowlist.
    #[cfg(test)]
    pub(super) fn exhaustiveness_guard(self) -> usize {
        match self {
            Self::WebhookSigningSecret => 0,
            Self::PersonalToken => 1,
            Self::OAuthAccessToken => 2,
            Self::OAuthRefreshToken => 3,
            Self::EndpointUrl => 4,
            Self::SmtpHost => 5,
            Self::SmtpPort => 6,
            Self::SmtpUsername => 7,
            Self::SmtpPassword => 8,
            Self::FromAddress => 9,
        }
    }

    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        match value {
            "webhook_signing_secret" => Ok(Self::WebhookSigningSecret),
            "personal_token" => Ok(Self::PersonalToken),
            "oauth_access_token" => Ok(Self::OAuthAccessToken),
            "oauth_refresh_token" => Ok(Self::OAuthRefreshToken),
            "endpoint_url" => Ok(Self::EndpointUrl),
            "smtp_host" => Ok(Self::SmtpHost),
            "smtp_port" => Ok(Self::SmtpPort),
            "smtp_username" => Ok(Self::SmtpUsername),
            "smtp_password" => Ok(Self::SmtpPassword),
            "from_address" => Ok(Self::FromAddress),
            _ => Err(DomainError::Storage),
        }
    }
}

impl fmt::Display for CredentialKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::WebhookSigningSecret => "webhook_signing_secret",
            Self::PersonalToken => "personal_token",
            Self::OAuthAccessToken => "oauth_access_token",
            Self::OAuthRefreshToken => "oauth_refresh_token",
            Self::EndpointUrl => "endpoint_url",
            Self::SmtpHost => "smtp_host",
            Self::SmtpPort => "smtp_port",
            Self::SmtpUsername => "smtp_username",
            Self::SmtpPassword => "smtp_password",
            Self::FromAddress => "from_address",
        })
    }
}
