\set ON_ERROR_STOP on

begin;

-- A presentation run is a replaceable snapshot, not an event generator. Remove
-- the incidents produced by its previous four inbound events before deleting
-- their delivery evidence. Static incidents from seed.sql are not selected.
delete from incidents
where team_id = :'team_id'::uuid
  and id in (
    select run.incident_id
    from automation_runs run
    join automation_rules rule on rule.id = run.rule_id
    where rule.team_id = :'team_id'::uuid
      and rule.name like 'Demo: %'
      and run.delivery_id in (
        select delivery.id
        from webhook_deliveries delivery
        join service_connections connection on connection.id = delivery.connection_id
        where connection.team_id = :'team_id'::uuid
          and delivery.provider_delivery_id in (
            :'github_delivery_id', :'gitlab_delivery_id',
            :'generic_delivery_id', :'alertmanager_delivery_id'
          )
      )
      and run.incident_id is not null
  );

-- automation_runs cascade with their owned delivery rows.
delete from webhook_deliveries delivery
using service_connections connection
where connection.id = delivery.connection_id
  and connection.team_id = :'team_id'::uuid
  and delivery.provider_delivery_id in (
    :'github_delivery_id', :'gitlab_delivery_id',
    :'generic_delivery_id', :'alertmanager_delivery_id'
  );

commit;
