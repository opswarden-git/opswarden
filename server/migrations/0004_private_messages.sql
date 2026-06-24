-- RTC2 private messages: strictly bilateral 1-to-1 direct messages between two
-- users. Not tied to an incident/release/team — the conversation is keyed only
-- by its two participants. Both participants read the same history; the stored
-- (sender_id, recipient_id) keeps authorship unambiguous while reads fetch both
-- directions. Content length is bounded in the domain (`PrivateMessage`, 2000
-- chars); the column itself stays plain `text`.
--
-- ON DELETE CASCADE on both participants: deleting an account removes that
-- user's private messages. The conversation history therefore only lives while
-- both participants exist — an accepted simplification for now (account deletion
-- is already guarded elsewhere), revisited if PM retention becomes a concern.
CREATE TABLE IF NOT EXISTS private_messages (
    id           uuid        PRIMARY KEY,
    sender_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    recipient_id uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    content      text        NOT NULL,
    created_at   timestamptz NOT NULL
);

-- A conversation read fetches both directions between two users, newest first.
-- Two directed indexes cover both legs of the `(a->b) OR (b->a)` filter.
CREATE INDEX IF NOT EXISTS idx_private_messages_sender_recipient
    ON private_messages (sender_id, recipient_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_private_messages_recipient_sender
    ON private_messages (recipient_id, sender_id, created_at DESC);
