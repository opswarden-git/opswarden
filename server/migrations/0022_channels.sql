-- Channels — Real-time chat instances scoped to a team.

create table if not exists channels (
    id uuid primary key,
    team_id uuid not null references teams (id) on delete cascade,
    name text not null,
    created_at timestamptz not null,
    unique(team_id, name)
);

create index if not exists channels_team_created_at_idx
    on channels (team_id, created_at asc);
