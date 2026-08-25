-- opswarden: migration-phase=expand
-- Rich bilateral messaging. Attachments remain deliberately bounded and live
-- with the message so authorization, deletion and backup stay atomic.

alter table private_messages
    add column if not exists edited_at timestamptz;

create table if not exists private_message_attachments (
    id uuid primary key,
    message_id uuid not null references private_messages (id) on delete cascade,
    file_name text not null check (char_length(file_name) between 1 and 255),
    media_type text not null check (char_length(media_type) between 1 and 127),
    content bytea not null check (octet_length(content) between 1 and 5242880),
    created_at timestamptz not null
);

create index if not exists private_message_attachments_message_idx
    on private_message_attachments (message_id, created_at, id);

create table if not exists private_message_reactions (
    message_id uuid not null references private_messages (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    emoji text not null check (char_length(emoji) between 1 and 16),
    created_at timestamptz not null,
    primary key (message_id, user_id, emoji)
);

create index if not exists private_message_reactions_message_idx
    on private_message_reactions (message_id, emoji);
