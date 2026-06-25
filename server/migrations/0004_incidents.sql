-- Incidents — lifecycle state tracked inside a team workspace.

create table if not exists incidents (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    title text not null,
    status text not null check (status in ('open', 'acknowledged', 'escalated', 'resolved')),
    severity text not null check (severity in ('low', 'medium', 'high', 'critical')),
    -- Responder assigned by a Manager; nullable means unassigned.
    assignee_id uuid references users (id) on delete set null,
    created_at timestamptz not null
);

create index if not exists incidents_team_created_at_idx
    on incidents (team_id, created_at desc);
