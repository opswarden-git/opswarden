-- Permit a crashed webhook attempt to be retried without allowing the stale
-- worker to finalize a delivery after its lease has been reclaimed.
alter table webhook_deliveries
    add column if not exists claim_token uuid,
    add column if not exists claim_expires_at timestamptz;

-- Deliveries abandoned before this migration become immediately reclaimable.
update webhook_deliveries
set claim_token = gen_random_uuid(),
    claim_expires_at = now()
where status = 'received'
  and claim_token is null;
