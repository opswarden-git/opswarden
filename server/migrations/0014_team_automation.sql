-- R9.1 — Team-scoped service connections and durable automation.
--
-- This migration is deliberately additive. The legacy `external_secrets` table
-- and boot-configured rules remain available until all HTTP and webhook
-- consumers move to these resources in later R9 batches.

create table if not exists service_connections (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    service text not null check (
        char_length(trim(service)) between 1 and 100
    ),
    created_by uuid references users (id) on delete set null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (team_id, service),
    -- Required by the composite rule foreign keys below. It makes a rule's
    -- same-Team ownership a storage invariant rather than a caller convention.
    unique (id, team_id)
);

create index if not exists service_connections_team_updated_at_idx
    on service_connections (team_id, updated_at desc, id);

-- Credential material is kept out of the connection metadata row. Listing a
-- Team's connections therefore never needs to select nonce or ciphertext.
-- Multiple kinds support providers that need both a webhook signing secret and
-- an OAuth/PAT credential without adding provider-specific columns.
create table if not exists service_connection_secrets (
    connection_id uuid not null
        references service_connections (id) on delete cascade,
    kind text not null check (kind in (
        'webhook_signing_secret',
        'personal_token',
        'oauth_access_token',
        'oauth_refresh_token',
        'endpoint_url'
    )),
    nonce bytea not null check (octet_length(nonce) = 12),
    ciphertext bytea not null check (octet_length(ciphertext) >= 16),
    updated_at timestamptz not null default now(),
    primary key (connection_id, kind)
);

create table if not exists automation_rules (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    name text not null check (char_length(trim(name)) between 1 and 200),
    enabled boolean not null default false,
    trigger_connection_id uuid not null,
    trigger_kind text not null check (
        char_length(trim(trigger_kind)) between 1 and 100
    ),
    trigger_config jsonb not null default '{}'::jsonb check (
        jsonb_typeof(trigger_config) = 'object'
    ),
    reaction_kind text not null check (
        char_length(trim(reaction_kind)) between 1 and 100
    ),
    reaction_connection_id uuid,
    reaction_config jsonb not null default '{}'::jsonb check (
        jsonb_typeof(reaction_config) = 'object'
    ),
    created_by uuid references users (id) on delete set null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (team_id, name),
    unique (id, team_id),
    foreign key (trigger_connection_id, team_id)
        references service_connections (id, team_id) on delete restrict,
    foreign key (reaction_connection_id, team_id)
        references service_connections (id, team_id) on delete restrict
);

create index if not exists automation_rules_team_updated_at_idx
    on automation_rules (team_id, updated_at desc, id);

create index if not exists automation_rules_enabled_trigger_idx
    on automation_rules (team_id, trigger_connection_id, trigger_kind)
    where enabled;

-- Provider delivery ids form the idempotency boundary. Reserving this row with
-- `ON CONFLICT DO NOTHING` happens before any reaction is executed.
create table if not exists webhook_deliveries (
    id uuid primary key,
    connection_id uuid not null
        references service_connections (id) on delete cascade,
    provider_delivery_id text not null check (
        char_length(trim(provider_delivery_id)) between 1 and 255
    ),
    provider_event text not null check (
        char_length(trim(provider_event)) between 1 and 100
    ),
    status text not null check (status in (
        'received', 'ignored', 'processed', 'failed'
    )),
    error_code text,
    received_at timestamptz not null default now(),
    unique (connection_id, provider_delivery_id)
);

create index if not exists webhook_deliveries_connection_received_at_idx
    on webhook_deliveries (connection_id, received_at desc, id desc);

-- One run per rule and provider delivery. Rules may later be deleted while the
-- operational evidence remains, hence `rule_id` is nullable on delete.
create table if not exists automation_runs (
    id uuid primary key,
    delivery_id uuid not null
        references webhook_deliveries (id) on delete cascade,
    rule_id uuid references automation_rules (id) on delete set null,
    status text not null check (status in (
        'running', 'succeeded', 'failed', 'skipped'
    )),
    incident_id uuid references incidents (id) on delete set null,
    error_code text,
    started_at timestamptz not null default now(),
    finished_at timestamptz,
    unique (delivery_id, rule_id),
    check (
        (status = 'running' and finished_at is null)
        or (status <> 'running' and finished_at is not null)
    )
);

create index if not exists automation_runs_rule_started_at_idx
    on automation_runs (rule_id, started_at desc, id desc);

create index if not exists automation_runs_delivery_id_idx
    on automation_runs (delivery_id);
