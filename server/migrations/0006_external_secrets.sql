-- External secrets — encrypted third-party credentials.
--
-- AES-256-GCM: only the per-row nonce and ciphertext are stored, so a raw
-- `SELECT * FROM external_secrets` reveals nothing usable. Keyed by service
-- name ("github", later "slack", ...).

create table if not exists external_secrets (
    service text primary key,
    nonce bytea not null,
    ciphertext bytea not null,
    updated_at timestamptz not null default now()
);

