-- Durable incident detail and reconstructible system activity.

alter table incidents
    add column if not exists description text not null default '',
    add column if not exists created_by uuid references users (id) on delete set null,
    add column if not exists updated_at timestamptz;

update incidents
set updated_at = created_at
where updated_at is null;

alter table incidents
    alter column updated_at set default now(),
    alter column updated_at set not null;

create table if not exists incident_events (
    id uuid primary key,
    incident_id uuid not null references incidents (id) on delete cascade,
    kind text not null check (
        kind in ('created', 'status_changed', 'assigned', 'severity_changed')
    ),
    actor_id uuid references users (id) on delete set null,
    data jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now()
);

create index if not exists incident_events_incident_created_at_idx
    on incident_events (incident_id, created_at desc, id desc);

-- Operational notes outlive user accounts. Their content remains useful while
-- the author is deliberately pseudonymized by the nullable foreign key.
alter table timeline_entries
    drop constraint if exists timeline_entries_author_id_fkey;

alter table timeline_entries
    alter column author_id drop not null;

alter table timeline_entries
    add constraint timeline_entries_author_id_fkey
    foreign key (author_id) references users (id) on delete set null;
