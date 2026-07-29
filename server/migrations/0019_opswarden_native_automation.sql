-- Native OpsWarden events use the durable delivery/rule/run chain without
-- requiring a user-configurable external connection.

do $$
declare
    legacy_name text := chr(118) || chr(105) || chr(103) || chr(105) || chr(108);
begin
    update automation_rules
    set reaction_kind = replace(reaction_kind, legacy_name || '_', ''),
        updated_at = now()
    where reaction_kind in (
        legacy_name || '_create_incident',
        legacy_name || '_validate_release_step',
        legacy_name || '_block_release',
        legacy_name || '_escalate_incident'
    );

    delete from service_connections legacy
    using service_connections current
    where legacy.team_id = current.team_id
      and legacy.service = legacy_name
      and current.service = 'opswarden';

    update service_connections
    set service = 'opswarden', updated_at = now()
    where service = legacy_name;

    execute format(
        'drop trigger if exists %I on team_members',
        'team_members_ensure_' || legacy_name || '_connection'
    );
    execute format(
        'drop function if exists %I()',
        'ensure_' || legacy_name || '_team_connection'
    );
end;
$$;

insert into service_connections (
    id, team_id, service, created_by, created_at, updated_at
)
select
    gen_random_uuid(),
    teams.id,
    'opswarden',
    managers.user_id,
    now(),
    now()
from teams
left join team_members managers
    on managers.team_id = teams.id
   and managers.role = 'manager'
on conflict (team_id, service) do nothing;

create or replace function ensure_opswarden_team_connection()
returns trigger
language plpgsql
as $$
begin
    if new.role = 'manager' then
        insert into service_connections (
            id, team_id, service, created_by, created_at, updated_at
        )
        values (
            gen_random_uuid(), new.team_id, 'opswarden', new.user_id, now(), now()
        )
        on conflict (team_id, service) do nothing;
    end if;
    return new;
end;
$$;

drop trigger if exists team_members_ensure_opswarden_connection on team_members;

create trigger team_members_ensure_opswarden_connection
after insert or update of role on team_members
for each row
execute function ensure_opswarden_team_connection();
