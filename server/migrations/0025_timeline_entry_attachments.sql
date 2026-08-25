-- opswarden: migration-phase=expand
-- Incident-room attachments remain bounded and inherit the lifetime of their
-- operational note. Downloads are authorized through Team membership.

create table if not exists timeline_entry_attachments (
    id uuid primary key,
    entry_id uuid not null references timeline_entries (id) on delete cascade,
    file_name text not null check (char_length(file_name) between 1 and 255),
    media_type text not null check (char_length(media_type) between 1 and 127),
    content bytea not null check (octet_length(content) between 1 and 5242880),
    created_at timestamptz not null
);

create index if not exists timeline_entry_attachments_entry_idx
    on timeline_entry_attachments (entry_id, created_at, id);
