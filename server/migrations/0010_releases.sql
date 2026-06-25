-- Releases — planned deployments coordinated in real time.
--
-- `base_state` is the stored lifecycle (created/in_progress/completed/cancelled).
-- The effective `blocked` state is derived at read/emit time as:
-- base_state = in_progress AND >= 1 linked incident is not resolved.

create table if not exists releases (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    title text not null,
    base_state text not null check (base_state in ('created', 'in_progress', 'completed', 'cancelled')),
    created_at timestamptz not null
);

create index if not exists releases_team_created_at_idx
    on releases (team_id, created_at desc);

-- Ordered steps; a step validates only after the previous one.
-- `validated_by` survives kick/ban because moderation removes membership, not
-- the user account. ON DELETE SET NULL handles later account deletion.
create table if not exists release_steps (
    release_id uuid not null references releases (id) on delete cascade,
    position integer not null,
    name text not null,
    validated_by uuid references users (id) on delete set null,
    validated_at timestamptz,
    primary key (release_id, position)
);

-- Many-to-many incident links. A release is blocked while any linked incident is
-- active (status <> 'resolved'); it unblocks once all linked incidents resolve.
create table if not exists release_incidents (
    release_id uuid not null references releases (id) on delete cascade,
    incident_id uuid not null references incidents (id) on delete cascade,
    primary key (release_id, incident_id)
);

create index if not exists release_incidents_incident_id_idx
    on release_incidents (incident_id);
