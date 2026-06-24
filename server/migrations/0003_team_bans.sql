-- RTC2 moderation: per-team bans that block (re)joining.
-- One ban row per (team, user) — the composite primary key is the unique
-- constraint, so re-banning upserts rather than duplicating. A NULL expires_at
-- means a permanent ban; a non-NULL value is a temporary ban that stops
-- blocking once it passes.
CREATE TABLE IF NOT EXISTS team_bans (
    team_id    uuid        NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at timestamptz,
    reason     text,
    -- The moderator who issued the ban. Nullable + ON DELETE SET NULL so deleting
    -- the moderator's account later never fails on this FK; the ban record (and
    -- its block) survives, it just loses the "issued by" attribution.
    created_by uuid        REFERENCES users(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_team_bans_team ON team_bans (team_id);
