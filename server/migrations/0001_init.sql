-- OpsWarden — schema dictionary (single init migration).
--
-- One file, every table. We deliberately keep the whole schema here rather than
-- splitting it across many migrations: it reads as a dictionary of the domain's
-- persisted state, and sqlx compiles every `query!` against it in one place.
-- New tables get a new commented section below, not a new file.

-- ============================================================================
-- Users — authentication identities (see `domain::user`).
-- ============================================================================
create table if not exists users (
    id uuid primary key,
    email text unique not null,
    password_hash text not null,
    created_at timestamptz not null default now()
);

-- ============================================================================
-- Teams & RBAC — teams and their role-scoped membership (see `domain::team`).
-- ============================================================================
create table if not exists teams (
    id uuid primary key,
    name text not null,
    -- Human-friendly join handle, e.g. `OPS-A7B9X2` (see `InvitationCode`).
    invitation_code text unique not null,
    created_at timestamptz not null default now()
);

create table if not exists team_members (
    team_id uuid not null references teams (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    -- RBAC role; mirrors the `domain::team::Role` enum.
    role text not null check (role in ('observer', 'responder', 'manager')),
    joined_at timestamptz not null default now(),
    -- A user holds at most one role per team.
    primary key (team_id, user_id)
);

-- Single-Manager invariant enforced at the storage layer: a team may hold at
-- most one Manager. Mirrors `domain::team::plan_manager_transfer`, so a faulty
-- non-atomic write can never produce two Managers.
create unique index if not exists one_manager_per_team
    on team_members (team_id)
    where role = 'manager';

-- ============================================================================
-- Auth sessions — revoked bearer tokens kept until natural expiration.
-- ============================================================================
create table if not exists revoked_tokens (
    token_hash text primary key,
    expires_at timestamptz not null,
    revoked_at timestamptz not null default now()
);

create index if not exists revoked_tokens_expires_at_idx
    on revoked_tokens (expires_at);

-- ============================================================================
-- Incidents — lifecycle state tracked inside a team workspace.
-- ============================================================================
create table if not exists incidents (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    title text not null,
    status text not null check (status in ('open', 'acknowledged', 'escalated', 'resolved')),
    severity text not null check (severity in ('low', 'medium', 'high', 'critical')),
    -- Responder assigned by a Manager; nullable (unassigned), kept on user delete.
    assignee_id uuid references users (id) on delete set null,
    created_at timestamptz not null
);

create index if not exists incidents_team_created_idx
    on incidents (team_id, created_at desc);

-- ============================================================================
-- Incident timeline — timestamped entries authored by team members.
-- ============================================================================
create table if not exists timeline_entries (
    id uuid primary key,
    incident_id uuid not null references incidents (id) on delete cascade,
    author_id uuid not null references users (id) on delete restrict,
    content text not null,
    created_at timestamptz not null
);

create index if not exists timeline_entries_incident_created_idx
    on timeline_entries (incident_id, created_at desc);

-- ============================================================================
-- External secrets — encrypted third-party credentials (see `ports::SecretVault`
-- / `adapters::pg::vault`). AES-256-GCM: only the per-row nonce and ciphertext
-- are stored, so a raw `SELECT * FROM external_secrets` reveals nothing usable.
-- Keyed by service name ("github", later "slack", …).
-- ============================================================================
create table if not exists external_secrets (
    service text primary key,
    nonce bytea not null,
    ciphertext bytea not null,
    updated_at timestamptz not null default now()
);
