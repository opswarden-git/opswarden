-- Auth sessions — revoked bearer tokens kept until natural expiration.

create table if not exists revoked_tokens (
    token_hash text primary key,
    expires_at timestamptz not null,
    revoked_at timestamptz not null default now()
);

create index if not exists revoked_tokens_expires_at_idx
    on revoked_tokens (expires_at);

