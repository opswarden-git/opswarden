-- R9.3 — durable provider-verification and delivery health.
--
-- A signed provider request (including GitHub `ping`) verifies the connection.
-- Keeping this on the metadata row lets the future UI report real state without
-- loading delivery logs or credential material.

alter table service_connections
    add column if not exists verified_at timestamptz,
    add column if not exists last_delivery_at timestamptz,
    add column if not exists last_error_code text;
