-- opswarden: migration-phase=expand
-- Per-user read position for incident War Room channels. The position belongs
-- to the server so unread state survives refreshes and follows the user across
-- browsers and desktop clients.

create table if not exists incident_channel_reads (
    incident_id uuid not null references incidents (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    read_through timestamptz not null,
    primary key (incident_id, user_id)
);

create index if not exists incident_channel_reads_user_idx
    on incident_channel_reads (user_id, read_through desc);
