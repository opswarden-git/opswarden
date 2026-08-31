-- Every Team owns one credential-free internal VIGIL connection. It lets
-- native product events use the same durable delivery/rule/run chain as
-- external webhooks without pretending that they arrived over HTTP.

insert into service_connections (
    id, team_id, service, created_by, created_at, updated_at
)
select
    gen_random_uuid(),
    teams.id,
    'vigil',
    managers.user_id,
    now(),
    now()
from teams
left join team_members managers
    on managers.team_id = teams.id
   and managers.role = 'manager'
on conflict (team_id, service) do nothing;

create or replace function ensure_vigil_team_connection()
returns trigger
language plpgsql
as $$
begin
    if new.role = 'manager' then
        insert into service_connections (
            id, team_id, service, created_by, created_at, updated_at
        )
        values (
            gen_random_uuid(), new.team_id, 'vigil', new.user_id, now(), now()
        )
        on conflict (team_id, service) do nothing;
    end if;
    return new;
end;
$$;

drop trigger if exists team_members_ensure_vigil_connection on team_members;

create trigger team_members_ensure_vigil_connection
after insert or update of role on team_members
for each row
execute function ensure_vigil_team_connection();
