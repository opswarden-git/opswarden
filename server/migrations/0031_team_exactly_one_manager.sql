-- A team must have exactly one Manager at every transaction boundary.
-- Deferred checks allow an atomic demote/promote transfer and team creation.

create or replace function enforce_team_exactly_one_manager()
returns trigger
language plpgsql
as $$
declare
    checked_team_id uuid;
begin
    if tg_table_name = 'teams' then
        checked_team_id := new.id;
    elsif tg_op = 'DELETE' then
        checked_team_id := old.team_id;
    else
        checked_team_id := new.team_id;
    end if;

    if exists (select 1 from teams where id = checked_team_id)
       and (select count(*) from team_members
            where team_id = checked_team_id and role = 'manager') <> 1 then
        raise exception 'team % must have exactly one manager', checked_team_id
            using errcode = '23514', constraint = 'team_exactly_one_manager';
    end if;

    return null;
end;
$$;

create constraint trigger teams_exactly_one_manager_after_team
after insert or update on teams
deferrable initially deferred
for each row execute function enforce_team_exactly_one_manager();

create constraint trigger teams_exactly_one_manager_after_membership
after insert or update or delete on team_members
deferrable initially deferred
for each row execute function enforce_team_exactly_one_manager();

do $$
begin
    if exists (
        select 1
        from teams t
        where (select count(*) from team_members m
               where m.team_id = t.id and m.role = 'manager') <> 1
    ) then
        raise exception 'existing teams violate the exactly-one-manager invariant';
    end if;
end;
$$;
