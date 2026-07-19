-- R9.6 — the Team-owned connection/rule engine is now the only automation path.
--
-- A legacy secret has no Team ownership column, so assigning it automatically
-- would risk exposing one Team's credential to another. Operators must
-- reconnect the service explicitly through the owning Team before applying
-- this migration. The old ciphertext is then intentionally removed.

drop table if exists external_secrets;
