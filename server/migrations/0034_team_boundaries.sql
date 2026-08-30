-- opswarden: migration-phase=contract
-- Enforce tenant boundaries in PostgreSQL. Historical authors and validators
-- may outlive membership, so those relationships are checked when written;
-- current assignments and Release links remain valid for their whole lifetime.

alter table incidents
    add constraint incidents_team_id_id_key unique (team_id, id),
    add constraint incidents_team_assignee_fkey
        foreign key (team_id, assignee_id)
        references team_members (team_id, user_id)
        on delete set null (assignee_id);

alter table releases
    add constraint releases_team_id_id_key unique (team_id, id);

alter table release_incidents
    add column team_id uuid;

update release_incidents links
set team_id = releases.team_id
from releases
where releases.id = links.release_id;

-- Compatibility for old pods during a rolling update: their INSERT still
-- supplies only (release_id, incident_id). Derive the redundant tenant key
-- before NOT NULL and the composite foreign keys validate the relationship.
create or replace function fill_release_incident_team_id()
returns trigger
language plpgsql
as $$
begin
    if new.team_id is null then
        select team_id into new.team_id
        from releases
        where id = new.release_id;
    end if;
    return new;
end;
$$;

create trigger release_incidents_fill_team_id
before insert or update of release_id, team_id on release_incidents
for each row execute function fill_release_incident_team_id();

alter table release_incidents
    alter column team_id set not null,
    add constraint release_incidents_team_release_fkey
        foreign key (team_id, release_id)
        references releases (team_id, id)
        on delete cascade,
    add constraint release_incidents_team_incident_fkey
        foreign key (team_id, incident_id)
        references incidents (team_id, id)
        on delete cascade;

create or replace function enforce_timeline_author_team()
returns trigger
language plpgsql
as $$
begin
    if new.author_id is null
       or (tg_op = 'UPDATE' and new.author_id is not distinct from old.author_id
           and new.incident_id = old.incident_id) then
        return new;
    end if;

    perform 1
    from incidents incident
    join team_members member
      on member.team_id = incident.team_id
     and member.user_id = new.author_id
    where incident.id = new.incident_id
    for key share of incident, member;

    if not found then
        raise foreign_key_violation
            using constraint = 'timeline_entries_team_author_fkey',
                  message = 'timeline author must belong to the incident team';
    end if;
    return new;
end;
$$;

create trigger timeline_entries_enforce_author_team
before insert or update of incident_id, author_id on timeline_entries
for each row execute function enforce_timeline_author_team();

create or replace function enforce_release_step_validator_team()
returns trigger
language plpgsql
as $$
begin
    if new.validated_by is null
       or (tg_op = 'UPDATE' and new.validated_by is not distinct from old.validated_by
           and new.release_id = old.release_id) then
        return new;
    end if;

    perform 1
    from releases release
    join team_members member
      on member.team_id = release.team_id
     and member.user_id = new.validated_by
    where release.id = new.release_id
    for key share of release, member;

    if not found then
        raise foreign_key_violation
            using constraint = 'release_steps_team_validator_fkey',
                  message = 'release step validator must belong to the release team';
    end if;
    return new;
end;
$$;

create trigger release_steps_enforce_validator_team
before insert or update of release_id, validated_by on release_steps
for each row execute function enforce_release_step_validator_team();

-- A transaction may insert a ban before removing membership, or remove an
-- expired ban after joining. Serialize changes for one (team, user) pair and
-- validate the final transaction state with a deferred constraint trigger.
create or replace function lock_team_access_pair()
returns trigger
language plpgsql
as $$
begin
    perform pg_advisory_xact_lock(
        hashtextextended(new.team_id::text || ':' || new.user_id::text, 0)
    );
    return new;
end;
$$;

create trigger team_members_lock_access_pair
before insert or update of team_id, user_id on team_members
for each row execute function lock_team_access_pair();

create trigger team_bans_lock_access_pair
before insert or update of team_id, user_id, expires_at on team_bans
for each row execute function lock_team_access_pair();

create or replace function enforce_member_active_ban_exclusion()
returns trigger
language plpgsql
as $$
begin
    if exists (
        select 1
        from team_members member
        join team_bans ban
          on ban.team_id = member.team_id
         and ban.user_id = member.user_id
        where member.team_id = new.team_id
          and member.user_id = new.user_id
          and (ban.expires_at is null or ban.expires_at > current_timestamp)
    ) then
        raise check_violation
            using constraint = 'team_member_active_ban_exclusion',
                  message = 'a user cannot be both a team member and actively banned';
    end if;
    return null;
end;
$$;

create constraint trigger team_members_exclude_active_ban
after insert or update of team_id, user_id on team_members
deferrable initially deferred
for each row execute function enforce_member_active_ban_exclusion();

create constraint trigger team_bans_exclude_active_member
after insert or update of team_id, user_id, expires_at on team_bans
deferrable initially deferred
for each row execute function enforce_member_active_ban_exclusion();

do $$
begin
    if exists (
        select 1
        from team_members member
        join team_bans ban
          on ban.team_id = member.team_id
         and ban.user_id = member.user_id
        where ban.expires_at is null or ban.expires_at > current_timestamp
    ) then
        raise exception 'existing memberships overlap active bans'
            using errcode = '23514', constraint = 'team_member_active_ban_exclusion';
    end if;
end;
$$;
