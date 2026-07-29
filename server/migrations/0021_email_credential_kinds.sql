-- Allow the SMTP credential kinds of the Email REAction.
--
-- The Email vertical shipped in v1.0.7 with five new `CredentialKind` variants
-- but without this migration, so `service_connection_secrets_kind_check`
-- rejected every one of them. Configuring an Email connection failed with
-- `storage_error` in production while the whole test suite stayed green: the
-- HTTP-level tests store credentials through an in-memory vault, which enforces
-- no constraint at all.
--
-- The allowlist is kept rather than dropped: it is defence in depth against a
-- buggy writer. The drift it caused is now covered by a Postgres test that
-- round-trips every `CredentialKind::ALL` variant, so a future variant added
-- without extending this list fails CI instead of production.

alter table service_connection_secrets
    drop constraint if exists service_connection_secrets_kind_check;

alter table service_connection_secrets
    add constraint service_connection_secrets_kind_check check (kind in (
        'webhook_signing_secret',
        'personal_token',
        'oauth_access_token',
        'oauth_refresh_token',
        'endpoint_url',
        'smtp_host',
        'smtp_port',
        'smtp_username',
        'smtp_password',
        'from_address'
    ));
