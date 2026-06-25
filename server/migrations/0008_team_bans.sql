-- Team moderation — per-team bans that block rejoining.
--
-- One ban row per (team, user). A NULL expires_at means a permanent ban; a
-- non-NULL value is a temporary ban that stops blocking once it passes.

create table if not exists team_bans (
    team_id uuid not null references teams (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    expires_at timestamptz,
    reason text,
    -- Nullable + ON DELETE SET NULL so deleting the moderator's account later
    -- never fails on this FK; the ban survives without attribution.
    created_by uuid references users (id) on delete set null,
    created_at timestamptz not null default now(),
    primary key (team_id, user_id)
);

create index if not exists team_bans_team_id_idx
    on team_bans (team_id);
