-- opswarden: migration-phase=expand
-- A Team image is a bounded, authorization-gated identity resource. Keeping
-- the binary out of `teams` prevents directory reads from hydrating it.
CREATE TABLE team_images (
    team_id uuid PRIMARY KEY REFERENCES teams(id) ON DELETE CASCADE,
    media_type text NOT NULL CHECK (media_type IN ('image/jpeg', 'image/png', 'image/webp')),
    content bytea NOT NULL CHECK (octet_length(content) BETWEEN 1 AND 1048576),
    updated_at timestamptz NOT NULL DEFAULT now()
);
