-- Timeline edit + reactions.

-- Timeline editing — mark an entry as edited while preserving created_at.
alter table timeline_entries
    add column if not exists edited_at timestamptz;

-- Timeline reactions — persistent emoji reactions per timeline entry.
-- The composite primary key makes a (user, emoji) reaction on an entry unique,
-- so a user can never duplicate the same emoji on the same entry.
create table if not exists timeline_reactions (
    entry_id uuid not null references timeline_entries (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    emoji text not null,
    created_at timestamptz not null default now(),
    primary key (entry_id, user_id, emoji)
);

create index if not exists timeline_reactions_entry_id_idx
    on timeline_reactions (entry_id);
