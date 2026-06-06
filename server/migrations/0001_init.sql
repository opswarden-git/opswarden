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
