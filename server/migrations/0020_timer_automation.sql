-- Durable Timer schedules and occurrence claims.

insert into service_connections (
    id, team_id, service, created_by, created_at, updated_at
)
select
    gen_random_uuid(),
    teams.id,
    'timer',
    managers.user_id,
    now(),
    now()
from teams
left join team_members managers
    on managers.team_id = teams.id
   and managers.role = 'manager'
on conflict (team_id, service) do nothing;

create or replace function ensure_timer_team_connection()
returns trigger
language plpgsql
as $$
begin
    if new.role = 'manager' then
        insert into service_connections (
            id, team_id, service, created_by, created_at, updated_at
        )
        values (
            gen_random_uuid(), new.team_id, 'timer', new.user_id, now(), now()
        )
        on conflict (team_id, service) do nothing;
    end if;
    return new;
end;
$$;

drop trigger if exists team_members_ensure_timer_connection on team_members;

create trigger team_members_ensure_timer_connection
after insert or update of role on team_members
for each row
execute function ensure_timer_team_connection();

create table automation_timer_schedules (
    rule_id uuid primary key
        references automation_rules (id) on delete cascade,
    schedule_kind text not null check (
        schedule_kind in ('daily_at', 'every_minutes')
    ),
    timezone text not null check (
        char_length(trim(timezone)) between 1 and 100
    ),
    local_time time,
    interval_minutes integer,
    next_run_at timestamptz not null,
    rule_updated_at timestamptz not null,
    last_claimed_at timestamptz,
    updated_at timestamptz not null default now(),
    check (
        (schedule_kind = 'daily_at'
            and local_time is not null
            and interval_minutes is null)
        or
        (schedule_kind = 'every_minutes'
            and local_time is null
            and interval_minutes between 5 and 1440)
    )
);

create index automation_timer_schedules_due_idx
    on automation_timer_schedules (next_run_at, rule_id);

create table automation_timer_occurrences (
    rule_id uuid not null
        references automation_rules (id) on delete cascade,
    scheduled_for timestamptz not null,
    delivery_id uuid not null unique
        references webhook_deliveries (id) on delete cascade,
    schedule_kind text not null check (
        schedule_kind in ('daily_at', 'every_minutes')
    ),
    timezone text not null,
    local_time time,
    interval_minutes integer,
    rule_updated_at timestamptz not null,
    claimed_at timestamptz not null,
    execution_started_at timestamptz,
    primary key (rule_id, scheduled_for),
    check (
        (schedule_kind = 'daily_at'
            and local_time is not null
            and interval_minutes is null)
        or
        (schedule_kind = 'every_minutes'
            and local_time is null
            and interval_minutes between 5 and 1440)
    )
);

create index automation_timer_occurrences_unstarted_idx
    on automation_timer_occurrences (claimed_at, rule_id)
    where execution_started_at is null;
