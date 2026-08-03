-- opswarden: migration-phase=contract
--
-- Drop the standalone Channel object.
--
-- Channels were built to satisfy the RTC chat grid literally. VIGIL never asks
-- for them: its WebSocket contract lists fifteen events and none is a channel
-- event, and the exemption clause requires prior achievements to be covered by
-- VIGIL deliverables rather than reimplemented beside them. In an incident
-- response platform the Incident already is the room — the industry builds the
-- channel from the incident, never next to it.
--
-- The RTC criteria stay answerable through VIGIL deliverables: chan_message is
-- timeline_entry_added broadcast over the socket, chan_create and chan_delete
-- follow the Incident lifecycle, and the incident queue is the room list. That
-- mapping is recorded in .other/docs/AUDIT_DECLARED_VS_IMPLEMENTED.md.
--
-- Safe to drop: the HTTP surface had no client. No component, route or query in
-- client-web ever referenced it, so no user could reach the data.

drop table if exists channels;
