-- opswarden: migration-phase=contract
--
-- Record release progress on the incidents a release is blocked by.
--
-- Validating a release step already publishes `ReleaseStepValidated` over the
-- WebSocket, but nothing was written to `incident_events`, so a war room reading
-- its own history could not tell that the release it blocks had moved. The
-- allowlist has to grow for the new kind to be storable at all.
--
-- Phase marker: `contract` rather than `expand`, because widening a CHECK means
-- replacing it, and the policy gate keys off the SQL verb (`drop constraint`)
-- rather than the intent. The effect is purely additive: every value accepted
-- before is still accepted.

alter table incident_events
    drop constraint if exists incident_events_kind_check;

alter table incident_events
    add constraint incident_events_kind_check check (kind in (
        'created',
        'status_changed',
        'assigned',
        'severity_changed',
        'release_step_validated'
    ));
