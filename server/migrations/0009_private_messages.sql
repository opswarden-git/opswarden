-- Private messages — bilateral 1-to-1 direct messages.
--
-- Not tied to an incident, release, or team. Authorization is enforced by the
-- app layer through shared-team membership; the stored pair keeps authorship
-- unambiguous while reads fetch both directions.

create table if not exists private_messages (
    id uuid primary key,
    sender_id uuid not null references users (id) on delete cascade,
    recipient_id uuid not null references users (id) on delete cascade,
    content text not null,
    created_at timestamptz not null
);

-- A conversation read fetches both directions between two users, newest first.
create index if not exists private_messages_sender_recipient_created_at_idx
    on private_messages (sender_id, recipient_id, created_at desc);

create index if not exists private_messages_recipient_sender_created_at_idx
    on private_messages (recipient_id, sender_id, created_at desc);
