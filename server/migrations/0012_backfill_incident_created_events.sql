-- Give pre-R5 incidents the same reconstructible starting point as new ones.
insert into incident_events (id, incident_id, kind, actor_id, data, created_at)
select
    gen_random_uuid(),
    incident.id,
    'created',
    incident.created_by,
    jsonb_build_object(
        'status', 'open',
        'severity', incident.severity,
        'backfilled', true
    ),
    incident.created_at
from incidents incident
where not exists (
    select 1
    from incident_events event
    where event.incident_id = incident.id
      and event.kind = 'created'
);
