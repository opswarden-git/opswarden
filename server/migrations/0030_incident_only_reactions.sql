-- opswarden: migration-phase=contract
-- VIGIL scopes emoji reactions to Incident timeline entries. Private messages
-- remain editable and support attachments, but no longer persist reactions.

drop table if exists private_message_reactions;
