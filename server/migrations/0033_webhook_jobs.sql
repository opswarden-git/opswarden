create table if not exists webhook_jobs (
    id uuid primary key,
    connection_id uuid not null references service_connections (id) on delete cascade,
    expected_service text not null
        check (char_length(trim(expected_service)) between 1 and 100),
    provider_delivery_id text not null
        check (char_length(trim(provider_delivery_id)) between 1 and 255),
    provider_event text not null
        check (char_length(trim(provider_event)) between 1 and 100),
    body bytea not null check (octet_length(body) between 0 and 1048576),
    status text not null default 'queued'
        check (status in ('queued', 'processing', 'completed')),
    attempts integer not null default 0 check (attempts >= 0),
    available_at timestamptz not null default now(),
    claim_token uuid,
    claim_expires_at timestamptz,
    last_error_code text,
    created_at timestamptz not null default now(),
    completed_at timestamptz,
    unique (connection_id, provider_delivery_id),
    check ((claim_token is null) = (claim_expires_at is null))
);

create index if not exists webhook_jobs_claim_idx
    on webhook_jobs (available_at, created_at, id)
    where status in ('queued', 'processing');
