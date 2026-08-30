\set ON_ERROR_STOP on

begin;

-- This fixture owns only the deterministic UUIDs below. The Team itself comes
-- from the real onboarding flow and is never created or renamed here.
delete from private_messages where id::text like '55000000-0000-4000-8000-%';
delete from releases where id::text like '54000000-0000-4000-8000-%';
delete from incidents where id::text like '51000000-0000-4000-8000-%';

-- Keep one real Manager and add the two operational roles. The contractor is
-- intentionally outside the Team so the ban surface has an honest example.
delete from team_bans
where team_id = :'team_id'::uuid
  and user_id in (:'responder_id'::uuid, :'observer_id'::uuid);

insert into team_members (team_id, user_id, role, joined_at) values
  (:'team_id'::uuid, :'responder_id'::uuid, 'responder', now() - interval '45 days'),
  (:'team_id'::uuid, :'observer_id'::uuid, 'observer', now() - interval '44 days')
on conflict (team_id, user_id) do update set
  role = excluded.role,
  joined_at = excluded.joined_at;

delete from team_members
where team_id = :'team_id'::uuid and user_id = :'contractor_id'::uuid;

insert into team_bans (team_id, user_id, expires_at, reason, created_by, created_at)
values (
  :'team_id'::uuid,
  :'contractor_id'::uuid,
  now() + interval '7 days',
  'Demo: temporary access suspended after the production review',
  :'manager_id'::uuid,
  now() - interval '1 day'
)
on conflict (team_id, user_id) do update set
  expires_at = excluded.expires_at,
  reason = excluded.reason,
  created_by = excluded.created_by,
  created_at = excluded.created_at;

insert into incidents (
  id, team_id, title, description, status, severity,
  assignee_id, created_by, created_at, updated_at
) values
  (
    '51000000-0000-4000-8000-000000000001', :'team_id'::uuid,
    'Payment API returning 502 in Europe',
    'Checkout requests in the European region exceed the error-budget threshold.',
    'escalated', 'critical', :'responder_id'::uuid, :'manager_id'::uuid,
    now() - interval '70 minutes', now() - interval '8 minutes'
  ),
  (
    '51000000-0000-4000-8000-000000000002', :'team_id'::uuid,
    'Checkout latency above SLO',
    'The p95 checkout latency is above 1.5 seconds for mobile traffic.',
    'acknowledged', 'high', :'responder_id'::uuid, :'observer_id'::uuid,
    now() - interval '4 hours', now() - interval '3 hours 35 minutes'
  ),
  (
    '51000000-0000-4000-8000-000000000003', :'team_id'::uuid,
    'Database connection saturation during flash sale',
    'The pool reached its configured ceiling during the launch window.',
    'resolved', 'critical', :'manager_id'::uuid, :'responder_id'::uuid,
    now() - interval '2 days', now() - interval '43 hours'
  ),
  (
    '51000000-0000-4000-8000-000000000004', :'team_id'::uuid,
    'SSO callback intermittent timeouts',
    'A subset of authentication callbacks timed out behind the edge proxy.',
    'resolved', 'medium', :'manager_id'::uuid, :'manager_id'::uuid,
    now() - interval '5 days', now() - interval '4 days 21 hours'
  ),
  (
    '51000000-0000-4000-8000-000000000005', :'team_id'::uuid,
    'CDN cache purge delay',
    'Purge propagation is delayed but customer traffic remains healthy.',
    'open', 'low', null, :'observer_id'::uuid,
    now() - interval '9 hours', now() - interval '9 hours'
  )
on conflict (id) do update set
  team_id = excluded.team_id,
  title = excluded.title,
  description = excluded.description,
  status = excluded.status,
  severity = excluded.severity,
  assignee_id = excluded.assignee_id,
  created_by = excluded.created_by,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at;

insert into incident_events (id, incident_id, kind, actor_id, data, created_at) values
  (
    '53000000-0000-4000-8000-000000000001',
    '51000000-0000-4000-8000-000000000001', 'created', :'manager_id'::uuid,
    '{"status":"open","severity":"critical"}', now() - interval '70 minutes'
  ),
  (
    '53000000-0000-4000-8000-000000000002',
    '51000000-0000-4000-8000-000000000001', 'assigned', :'manager_id'::uuid,
    jsonb_build_object('assignee_id', :'responder_id'), now() - interval '64 minutes'
  ),
  (
    '53000000-0000-4000-8000-000000000003',
    '51000000-0000-4000-8000-000000000001', 'status_changed', :'responder_id'::uuid,
    '{"from":"open","to":"acknowledged"}', now() - interval '61 minutes'
  ),
  (
    '53000000-0000-4000-8000-000000000004',
    '51000000-0000-4000-8000-000000000001', 'status_changed', :'manager_id'::uuid,
    '{"from":"acknowledged","to":"escalated"}', now() - interval '18 minutes'
  ),
  (
    '53000000-0000-4000-8000-000000000005',
    '51000000-0000-4000-8000-000000000002', 'created', :'observer_id'::uuid,
    '{"status":"open","severity":"high"}', now() - interval '4 hours'
  ),
  (
    '53000000-0000-4000-8000-000000000006',
    '51000000-0000-4000-8000-000000000003', 'created', :'responder_id'::uuid,
    '{"status":"open","severity":"critical"}', now() - interval '2 days'
  ),
  (
    '53000000-0000-4000-8000-000000000007',
    '51000000-0000-4000-8000-000000000003', 'status_changed', :'manager_id'::uuid,
    '{"from":"escalated","to":"resolved"}', now() - interval '43 hours'
  ),
  (
    '53000000-0000-4000-8000-000000000008',
    '51000000-0000-4000-8000-000000000004', 'created', :'manager_id'::uuid,
    '{"status":"open","severity":"medium"}', now() - interval '5 days'
  ),
  (
    '53000000-0000-4000-8000-000000000009',
    '51000000-0000-4000-8000-000000000004', 'status_changed', :'manager_id'::uuid,
    '{"from":"acknowledged","to":"resolved"}', now() - interval '4 days 21 hours'
  ),
  (
    '53000000-0000-4000-8000-000000000010',
    '51000000-0000-4000-8000-000000000005', 'created', :'observer_id'::uuid,
    '{"status":"open","severity":"low"}', now() - interval '9 hours'
  )
on conflict (id) do update set
  incident_id = excluded.incident_id,
  kind = excluded.kind,
  actor_id = excluded.actor_id,
  data = excluded.data,
  created_at = excluded.created_at;

insert into timeline_entries (id, incident_id, author_id, content, created_at, edited_at) values
  (
    '52000000-0000-4000-8000-000000000001',
    '51000000-0000-4000-8000-000000000001', :'manager_id'::uuid,
    'Incident command opened. We are freezing payment deploys until the error rate stabilizes.',
    now() - interval '68 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000002',
    '51000000-0000-4000-8000-000000000001', :'responder_id'::uuid,
    'Traffic shifted away from the unhealthy pool. Error rate is falling from 18% to 4%.',
    now() - interval '52 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000003',
    '51000000-0000-4000-8000-000000000001', :'observer_id'::uuid,
    'Customer support has the incident reference and a status-page update is ready.',
    now() - interval '39 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000004',
    '51000000-0000-4000-8000-000000000001', :'responder_id'::uuid,
    'Root cause isolated to a stale payment-provider route. The corrected configuration is in staging.',
    now() - interval '24 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000005',
    '51000000-0000-4000-8000-000000000001', :'manager_id'::uuid,
    'Escalated for the production promotion. Final smoke tests are the remaining gate.',
    now() - interval '16 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000006',
    '51000000-0000-4000-8000-000000000002', :'responder_id'::uuid,
    'The slow query is identified; an index-only mitigation is being validated.',
    now() - interval '3 hours 40 minutes', null
  ),
  (
    '52000000-0000-4000-8000-000000000007',
    '51000000-0000-4000-8000-000000000003', :'manager_id'::uuid,
    'Pool limits were raised and verified under replay traffic. Closing after the observation window.',
    now() - interval '43 hours', null
  )
on conflict (id) do update set
  incident_id = excluded.incident_id,
  author_id = excluded.author_id,
  content = excluded.content,
  created_at = excluded.created_at,
  edited_at = excluded.edited_at;

insert into timeline_reactions (entry_id, user_id, emoji, created_at) values
  ('52000000-0000-4000-8000-000000000002', :'manager_id'::uuid, '✅', now() - interval '50 minutes'),
  ('52000000-0000-4000-8000-000000000003', :'responder_id'::uuid, '👍', now() - interval '37 minutes'),
  ('52000000-0000-4000-8000-000000000004', :'observer_id'::uuid, '👀', now() - interval '22 minutes')
on conflict (entry_id, user_id, emoji) do update set created_at = excluded.created_at;

insert into releases (id, team_id, title, base_state, created_at, updated_at) values
  (
    '54000000-0000-4000-8000-000000000001', :'team_id'::uuid,
    'v2.8.0 — Payment resilience', 'in_progress',
    now() - interval '1 day', now() - interval '18 minutes'
  ),
  (
    '54000000-0000-4000-8000-000000000002', :'team_id'::uuid,
    'v2.7.3 — Authentication hotfix', 'completed',
    now() - interval '5 days', now() - interval '4 days 21 hours'
  ),
  (
    '54000000-0000-4000-8000-000000000003', :'team_id'::uuid,
    'v2.9.0 — Observability foundations', 'created',
    now() - interval '6 hours', now() - interval '6 hours'
  )
on conflict (id) do update set
  team_id = excluded.team_id,
  title = excluded.title,
  base_state = excluded.base_state,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at;

insert into release_steps (release_id, position, name, validated_by, validated_at) values
  ('54000000-0000-4000-8000-000000000001', 0, 'Build and sign artifacts', :'manager_id'::uuid, now() - interval '23 hours'),
  ('54000000-0000-4000-8000-000000000001', 1, 'Deploy to staging', :'responder_id'::uuid, now() - interval '20 hours'),
  ('54000000-0000-4000-8000-000000000001', 2, 'Run payment smoke tests', null, null),
  ('54000000-0000-4000-8000-000000000001', 3, 'Promote to production', null, null),
  ('54000000-0000-4000-8000-000000000002', 0, 'Build authentication hotfix', :'manager_id'::uuid, now() - interval '4 days 23 hours'),
  ('54000000-0000-4000-8000-000000000002', 1, 'Validate OAuth callbacks', :'responder_id'::uuid, now() - interval '4 days 22 hours'),
  ('54000000-0000-4000-8000-000000000002', 2, 'Deploy globally', :'manager_id'::uuid, now() - interval '4 days 21 hours'),
  ('54000000-0000-4000-8000-000000000003', 0, 'Publish dashboards', null, null),
  ('54000000-0000-4000-8000-000000000003', 1, 'Enable tracing sampler', null, null),
  ('54000000-0000-4000-8000-000000000003', 2, 'Validate alert routes', null, null)
on conflict (release_id, position) do update set
  name = excluded.name,
  validated_by = excluded.validated_by,
  validated_at = excluded.validated_at;

insert into release_incidents (release_id, incident_id) values
  ('54000000-0000-4000-8000-000000000001', '51000000-0000-4000-8000-000000000001'),
  ('54000000-0000-4000-8000-000000000002', '51000000-0000-4000-8000-000000000004')
on conflict (release_id, incident_id) do nothing;

insert into incident_events (id, incident_id, kind, actor_id, data, created_at) values
  (
    '53000000-0000-4000-8000-000000000011',
    '51000000-0000-4000-8000-000000000001', 'release_step_validated', :'manager_id'::uuid,
    '{"release_id":"54000000-0000-4000-8000-000000000001","release_title":"v2.8.0 — Payment resilience","position":0,"step":"Build and sign artifacts"}',
    now() - interval '23 hours'
  ),
  (
    '53000000-0000-4000-8000-000000000012',
    '51000000-0000-4000-8000-000000000001', 'release_step_validated', :'responder_id'::uuid,
    '{"release_id":"54000000-0000-4000-8000-000000000001","release_title":"v2.8.0 — Payment resilience","position":1,"step":"Deploy to staging"}',
    now() - interval '20 hours'
  )
on conflict (id) do update set
  incident_id = excluded.incident_id,
  kind = excluded.kind,
  actor_id = excluded.actor_id,
  data = excluded.data,
  created_at = excluded.created_at;

insert into private_messages (id, sender_id, recipient_id, content, created_at, edited_at) values
  (
    '55000000-0000-4000-8000-000000000001', :'manager_id'::uuid, :'responder_id'::uuid,
    'Can you take incident command for the payment API while I coordinate the release gate?',
    now() - interval '66 minutes', null
  ),
  (
    '55000000-0000-4000-8000-000000000002', :'responder_id'::uuid, :'manager_id'::uuid,
    'On it. Traffic shift is complete and I am posting every operational change in the war room.',
    now() - interval '63 minutes', null
  ),
  (
    '55000000-0000-4000-8000-000000000003', :'manager_id'::uuid, :'observer_id'::uuid,
    'Please keep the customer-impact summary aligned with the incident timeline.',
    now() - interval '41 minutes', null
  )
on conflict (id) do update set
  sender_id = excluded.sender_id,
  recipient_id = excluded.recipient_id,
  content = excluded.content,
  created_at = excluded.created_at,
  edited_at = excluded.edited_at;

commit;
