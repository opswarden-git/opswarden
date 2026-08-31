\set ON_ERROR_STOP on

begin;

-- The asynchronous webhook queue was introduced after the original delivery
-- ledger. Keep its cleanup separate so demo.py also supports deployments that
-- have not rolled out this expand migration yet.
delete from webhook_jobs job
using service_connections connection
where connection.id = job.connection_id
  and connection.team_id = :'team_id'::uuid
  and job.provider_delivery_id in (
    :'github_delivery_id', :'gitlab_delivery_id',
    :'generic_delivery_id', :'alertmanager_delivery_id'
  );

commit;
