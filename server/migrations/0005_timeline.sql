-- Incident timeline — timestamped entries authored by team members.

create table if not exists timeline_entries (
    id uuid primary key,
    incident_id uuid not null references incidents (id) on delete cascade,
    author_id uuid not null references users (id) on delete restrict,
    content text not null,
    created_at timestamptz not null
);

create index if not exists timeline_entries_incident_created_at_idx
    on timeline_entries (incident_id, created_at desc);
