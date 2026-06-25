-- VIGIL Phase 1 core: Releases — planned deployments coordinated in real time,
-- composed of sequential validated steps, that an active linked Incident blocks.
--
-- `base_state` is the stored lifecycle (created/in_progress/completed/cancelled).
-- The effective `blocked` state is NEVER stored: it is derived at read/emit time
-- as "base_state = in_progress AND >= 1 linked incident is not resolved", so a
-- release auto-unblocks the moment its last active linked incident resolves.
CREATE TABLE IF NOT EXISTS releases (
    id         uuid        PRIMARY KEY,
    team_id    uuid        NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    title      text        NOT NULL,
    base_state text        NOT NULL CHECK (base_state IN ('created', 'in_progress', 'completed', 'cancelled')),
    created_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_releases_team ON releases (team_id, created_at DESC);

-- Ordered steps; a step validates only after the previous one. `validated_by`
-- keeps attribution and survives kick/ban (moderation only removes team
-- membership, not the user account). ON DELETE SET NULL so deleting the *account*
-- later never fails on this FK; the validation record (and its order) survives.
CREATE TABLE IF NOT EXISTS release_steps (
    release_id   uuid        NOT NULL REFERENCES releases (id) ON DELETE CASCADE,
    position     integer     NOT NULL,
    name         text        NOT NULL,
    validated_by uuid        REFERENCES users (id) ON DELETE SET NULL,
    validated_at timestamptz,
    PRIMARY KEY (release_id, position)
);

-- Many-to-many incident links. A release is blocked while ANY linked incident is
-- active (status <> 'resolved'); it unblocks once all linked incidents resolve.
CREATE TABLE IF NOT EXISTS release_incidents (
    release_id  uuid NOT NULL REFERENCES releases (id) ON DELETE CASCADE,
    incident_id uuid NOT NULL REFERENCES incidents (id) ON DELETE CASCADE,
    PRIMARY KEY (release_id, incident_id)
);

CREATE INDEX IF NOT EXISTS idx_release_incidents_incident ON release_incidents (incident_id);
