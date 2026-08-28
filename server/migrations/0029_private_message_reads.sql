-- opswarden: migration-phase=expand
-- Per-user read position for bilateral direct messages.
-- The position belongs to the server so unread state survives refreshes
-- and follows the user across browsers and desktop clients.

create table if not exists private_message_reads (
    viewer_id uuid not null references users (id) on delete cascade,
    peer_id uuid not null references users (id) on delete cascade,
    read_through timestamptz not null,
    primary key (viewer_id, peer_id)
);

create index if not exists private_message_reads_viewer_idx
    on private_message_reads (viewer_id, read_through desc);
