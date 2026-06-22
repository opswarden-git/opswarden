-- OpsWarden — RTC 2 timeline edit + reactions.
--
-- A separate versioned migration (not a new section in 0001): `sqlx::migrate!`
-- checksum-locks an applied migration, and the running dev database already has
-- 0001 applied with live data we must not drop. New incremental schema therefore
-- ships as its own file.

-- ============================================================================
-- Timeline editing — mark an entry as edited while preserving created_at.
-- ============================================================================
alter table timeline_entries
    add column if not exists edited_at timestamptz;

-- ============================================================================
-- Timeline reactions — persistent emoji reactions per timeline entry.
-- The composite primary key makes a (user, emoji) reaction on an entry unique,
-- so a user can never duplicate the same emoji on the same entry.
-- ============================================================================
create table if not exists timeline_reactions (
    entry_id uuid not null references timeline_entries (id) on delete cascade,
    user_id uuid not null references users (id) on delete cascade,
    emoji text not null,
    created_at timestamptz not null default now(),
    primary key (entry_id, user_id, emoji)
);

create index if not exists timeline_reactions_entry_idx
    on timeline_reactions (entry_id);
