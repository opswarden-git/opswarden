\set ON_ERROR_STOP on

begin;

delete from automation_runs
where rule_id in (
  select id from automation_rules
  where team_id = :'team_id'::uuid and name like 'Demo: %'
);

delete from automation_rules
where team_id = :'team_id'::uuid and name like 'Demo: %';

delete from private_messages where id::text like '55000000-0000-4000-8000-%';
delete from releases where id::text like '54000000-0000-4000-8000-%';
delete from incidents where id::text like '51000000-0000-4000-8000-%';

delete from team_bans
where team_id = :'team_id'::uuid and user_id = :'contractor_id'::uuid;

delete from team_members
where team_id = :'team_id'::uuid
  and user_id in (:'responder_id'::uuid, :'observer_id'::uuid);

commit;
