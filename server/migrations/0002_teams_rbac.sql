-- Teams & RBAC — teams and their role-scoped membership.

create table if not exists teams (
    id uuid primary key,
    name text not null,
    -- Human-friendly join handle, e.g. `OPS-A7B9X2`.
    invitation_code text unique not null,
    created_at timestamptz not null default now()
);

create table if not exists team_members (
    team_id uuid not null references teams (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    -- Mirrors the `domain::team::Role` enum.
    role text not null check (role in ('observer', 'responder', 'manager')),
    joined_at timestamptz not null default now(),
    primary key (team_id, user_id)
);

-- Storage-level guard for the single-Manager invariant.
create unique index if not exists team_members_one_manager_idx
    on team_members (team_id)
    where role = 'manager';
