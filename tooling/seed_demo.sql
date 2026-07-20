\set ON_ERROR_STOP on

begin;

-- Team dialog E2E scenarios create real workspaces. Remove only their
-- namespaced fixtures before restoring the deterministic demo directory.
delete from teams where name like 'E2E team dialog %';

-- Stable Team IDs make demo URLs, screenshots and webhook configuration
-- reproducible across runs. Existing rows are refreshed, never duplicated.
insert into teams (id, name, invitation_code, created_at) values
  ('39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'OpsWarden Demo', 'OPS-DEMO01', now() - interval '90 days'),
  ('6d1e8c20-b622-4d21-9b1b-111111111111', 'Production Europe', 'OPS-PROD01', now() - interval '60 days'),
  ('8b2f9d30-c733-4e32-8c2c-222222222222', 'Security Lab', 'OPS-SEC001', now() - interval '30 days')
on conflict (id) do update set
  name = excluded.name,
  invitation_code = excluded.invitation_code;

-- Restore the intended manager before upserting memberships. This only touches
-- the three deterministic demo Teams.
update team_members set role = 'responder'
where team_id = '39aa8884-22cc-4764-a9e7-7df7c7619ba6'
  and role = 'manager' and user_id <> :'manager_id'::uuid;
update team_members set role = 'responder'
where team_id = '6d1e8c20-b622-4d21-9b1b-111111111111'
  and role = 'manager' and user_id <> :'responder_id'::uuid;
update team_members set role = 'responder'
where team_id = '8b2f9d30-c733-4e32-8c2c-222222222222'
  and role = 'manager' and user_id <> :'observer_id'::uuid;

insert into team_members (team_id, user_id, role, joined_at) values
  ('39aa8884-22cc-4764-a9e7-7df7c7619ba6', :'manager_id', 'manager', now() - interval '90 days'),
  ('39aa8884-22cc-4764-a9e7-7df7c7619ba6', :'responder_id', 'responder', now() - interval '89 days'),
  ('39aa8884-22cc-4764-a9e7-7df7c7619ba6', :'observer_id', 'observer', now() - interval '88 days'),
  ('6d1e8c20-b622-4d21-9b1b-111111111111', :'responder_id', 'manager', now() - interval '60 days'),
  ('6d1e8c20-b622-4d21-9b1b-111111111111', :'manager_id', 'responder', now() - interval '59 days'),
  ('6d1e8c20-b622-4d21-9b1b-111111111111', :'observer_id', 'observer', now() - interval '58 days'),
  ('8b2f9d30-c733-4e32-8c2c-222222222222', :'observer_id', 'manager', now() - interval '30 days'),
  ('8b2f9d30-c733-4e32-8c2c-222222222222', :'responder_id', 'responder', now() - interval '29 days'),
  ('8b2f9d30-c733-4e32-8c2c-222222222222', :'manager_id', 'observer', now() - interval '28 days')
on conflict (team_id, user_id) do update set
  role = excluded.role,
  joined_at = excluded.joined_at;

-- Remove only artifacts emitted by the bundled webhook simulator. This keeps
-- `just demo` deterministic without touching unrelated local automation data.
delete from automation_runs
where rule_id in (
  select id from automation_rules
  where team_id = '39aa8884-22cc-4764-a9e7-7df7c7619ba6'
    and name = 'Demo: GitHub CI failure to incident'
);

delete from incidents
where team_id = '39aa8884-22cc-4764-a9e7-7df7c7619ba6'
  and (
    title in ('CI failed on GitHub', 'CI failed on opswarden/demo')
    or title like 'E2E dialog contract %'
  );

insert into incidents (id, team_id, title, status, severity, assignee_id, created_at) values
  ('10000000-0000-4000-8000-000000000001', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Payment API returning 502 in Europe', 'open', 'critical', :'responder_id', now() - interval '18 minutes'),
  ('10000000-0000-4000-8000-000000000002', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Checkout latency above SLO', 'acknowledged', 'high', :'responder_id', now() - interval '47 minutes'),
  ('10000000-0000-4000-8000-000000000003', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Primary database replication lag', 'escalated', 'critical', :'responder_id', now() - interval '2 hours'),
  ('10000000-0000-4000-8000-000000000004', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'GitHub Actions failing on main', 'open', 'high', null, now() - interval '3 hours'),
  ('10000000-0000-4000-8000-000000000005', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'SSO intermittent timeouts', 'acknowledged', 'medium', :'manager_id', now() - interval '5 hours'),
  ('10000000-0000-4000-8000-000000000006', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'CDN cache purge delay', 'escalated', 'medium', :'responder_id', now() - interval '8 hours'),
  ('10000000-0000-4000-8000-000000000007', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Customer export job stalled', 'open', 'medium', null, now() - interval '14 hours'),
  ('10000000-0000-4000-8000-000000000008', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Elevated worker memory usage', 'open', 'low', :'responder_id', now() - interval '21 hours'),
  ('10000000-0000-4000-8000-000000000009', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Certificate renewal completed', 'resolved', 'high', :'manager_id', now() - interval '2 days'),
  ('10000000-0000-4000-8000-000000000010', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Search indexing recovered', 'resolved', 'medium', :'responder_id', now() - interval '3 days'),
  ('10000000-0000-4000-8000-000000000011', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Scheduled database failover drill', 'resolved', 'low', :'responder_id', now() - interval '5 days'),
  ('10000000-0000-4000-8000-000000000012', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'Mobile release crash spike', 'escalated', 'critical', :'manager_id', now() - interval '7 hours'),
  ('10000000-0000-4000-8000-000000000021', '6d1e8c20-b622-4d21-9b1b-111111111111', 'EU edge TLS handshake failures', 'open', 'critical', :'manager_id', now() - interval '32 minutes'),
  ('10000000-0000-4000-8000-000000000022', '6d1e8c20-b622-4d21-9b1b-111111111111', 'Kafka consumer lag on billing', 'acknowledged', 'high', :'manager_id', now() - interval '4 hours'),
  ('10000000-0000-4000-8000-000000000023', '6d1e8c20-b622-4d21-9b1b-111111111111', 'Invoice PDF queue delay', 'resolved', 'medium', :'responder_id', now() - interval '1 day'),
  ('10000000-0000-4000-8000-000000000024', '6d1e8c20-b622-4d21-9b1b-111111111111', 'Kubernetes nodes under disk pressure', 'escalated', 'high', :'manager_id', now() - interval '6 hours'),
  ('10000000-0000-4000-8000-000000000031', '8b2f9d30-c733-4e32-8c2c-222222222222', 'Suspicious OAuth token reuse', 'open', 'high', :'responder_id', now() - interval '1 hour'),
  ('10000000-0000-4000-8000-000000000032', '8b2f9d30-c733-4e32-8c2c-222222222222', 'Privileged login anomaly', 'acknowledged', 'critical', :'observer_id', now() - interval '3 hours'),
  ('10000000-0000-4000-8000-000000000033', '8b2f9d30-c733-4e32-8c2c-222222222222', 'Dependency vulnerability triaged', 'resolved', 'medium', :'responder_id', now() - interval '2 days'),
  ('10000000-0000-4000-8000-000000000034', '8b2f9d30-c733-4e32-8c2c-222222222222', 'WAF rule false-positive spike', 'escalated', 'high', :'responder_id', now() - interval '9 hours')
on conflict (id) do update set
  team_id = excluded.team_id, title = excluded.title, status = excluded.status,
  severity = excluded.severity, assignee_id = excluded.assignee_id,
  created_at = excluded.created_at;

-- Browser tests deliberately exercise the real note composer. Keep the demo
-- seed replayable by removing only their clearly namespaced notes.
delete from timeline_entries where content like 'E2E operational update %';

insert into timeline_entries (id, incident_id, author_id, content, created_at, edited_at) values
  ('20000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000001', :'manager_id', 'Alert confirmed across eu-west checkout pods. Incident declared SEV-1.', now() - interval '17 minutes', null),
  ('20000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000001', :'responder_id', 'Traffic shifted 30% to the healthy payment pool. Error rate is falling.', now() - interval '12 minutes', null),
  ('20000000-0000-4000-8000-000000000003', '10000000-0000-4000-8000-000000000001', :'responder_id', 'Provider status page is green; comparing the last gateway deployment now.', now() - interval '7 minutes', now() - interval '6 minutes'),
  ('20000000-0000-4000-8000-000000000004', '10000000-0000-4000-8000-000000000002', :'manager_id', 'p95 crossed 1.8 s for three consecutive windows.', now() - interval '45 minutes', null),
  ('20000000-0000-4000-8000-000000000005', '10000000-0000-4000-8000-000000000002', :'responder_id', 'Acknowledged. Slow query sample points to promotion lookup fan-out.', now() - interval '39 minutes', null),
  ('20000000-0000-4000-8000-000000000006', '10000000-0000-4000-8000-000000000002', :'responder_id', 'Feature flag checkout-promotions-v2 reduced to 10%.', now() - interval '24 minutes', null),
  ('20000000-0000-4000-8000-000000000007', '10000000-0000-4000-8000-000000000003', :'responder_id', 'Replica lag reached 94 seconds on pg-eu-02.', now() - interval '118 minutes', null),
  ('20000000-0000-4000-8000-000000000008', '10000000-0000-4000-8000-000000000003', :'manager_id', 'Escalating to database on-call. Write-heavy export jobs are paused.', now() - interval '101 minutes', null),
  ('20000000-0000-4000-8000-000000000009', '10000000-0000-4000-8000-000000000003', :'responder_id', 'Replication recovered to 18 seconds; retaining escalation until under 5 seconds.', now() - interval '33 minutes', null),
  ('20000000-0000-4000-8000-000000000010', '10000000-0000-4000-8000-000000000004', :'manager_id', 'Main workflow fails during the integration-test shard.', now() - interval '175 minutes', null),
  ('20000000-0000-4000-8000-000000000011', '10000000-0000-4000-8000-000000000004', :'responder_id', 'Failure reproduced locally with the new PostgreSQL image.', now() - interval '149 minutes', null),
  ('20000000-0000-4000-8000-000000000012', '10000000-0000-4000-8000-000000000005', :'manager_id', 'Auth callback latency is isolated to the legacy identity region.', now() - interval '290 minutes', null),
  ('20000000-0000-4000-8000-000000000013', '10000000-0000-4000-8000-000000000005', :'responder_id', 'New sessions are healthy. Existing retry storm is draining.', now() - interval '211 minutes', null),
  ('20000000-0000-4000-8000-000000000014', '10000000-0000-4000-8000-000000000006', :'responder_id', 'Vendor acknowledged a degraded purge queue in Paris.', now() - interval '470 minutes', null),
  ('20000000-0000-4000-8000-000000000015', '10000000-0000-4000-8000-000000000007', :'manager_id', 'Three enterprise exports have exceeded the 45-minute threshold.', now() - interval '13 hours', null),
  ('20000000-0000-4000-8000-000000000016', '10000000-0000-4000-8000-000000000008', :'responder_id', 'Heap growth correlates with image processing batches.', now() - interval '20 hours', null),
  ('20000000-0000-4000-8000-000000000017', '10000000-0000-4000-8000-000000000009', :'manager_id', 'New certificate deployed and verified from all edge regions.', now() - interval '46 hours', null),
  ('20000000-0000-4000-8000-000000000018', '10000000-0000-4000-8000-000000000009', :'responder_id', 'Resolved after 60 minutes of clean TLS probes.', now() - interval '45 hours', null),
  ('20000000-0000-4000-8000-000000000019', '10000000-0000-4000-8000-000000000010', :'responder_id', 'Backlog drained and freshness is below two minutes.', now() - interval '70 hours', null),
  ('20000000-0000-4000-8000-000000000020', '10000000-0000-4000-8000-000000000011', :'manager_id', 'Failover completed inside the 90-second recovery objective.', now() - interval '118 hours', null),
  ('20000000-0000-4000-8000-000000000021', '10000000-0000-4000-8000-000000000012', :'observer_id', 'Support reports crash loops concentrated on Android 14.', now() - interval '410 minutes', null),
  ('20000000-0000-4000-8000-000000000022', '10000000-0000-4000-8000-000000000012', :'manager_id', 'Rollback approved; mobile release train is frozen.', now() - interval '396 minutes', null),
  ('20000000-0000-4000-8000-000000000023', '10000000-0000-4000-8000-000000000021', :'responder_id', 'Handshake failure is limited to two edge POPs.', now() - interval '29 minutes', null),
  ('20000000-0000-4000-8000-000000000024', '10000000-0000-4000-8000-000000000022', :'manager_id', 'Billing consumer lag peaked at 180k messages.', now() - interval '220 minutes', null),
  ('20000000-0000-4000-8000-000000000025', '10000000-0000-4000-8000-000000000024', :'manager_id', 'Two nodes cordoned. Storage team is investigating image growth.', now() - interval '330 minutes', null),
  ('20000000-0000-4000-8000-000000000026', '10000000-0000-4000-8000-000000000031', :'observer_id', 'Token fingerprint appeared from Paris and Singapore within four minutes.', now() - interval '55 minutes', null),
  ('20000000-0000-4000-8000-000000000027', '10000000-0000-4000-8000-000000000031', :'responder_id', 'Sessions revoked and affected account forced through re-authentication.', now() - interval '41 minutes', null),
  ('20000000-0000-4000-8000-000000000028', '10000000-0000-4000-8000-000000000032', :'observer_id', 'Break-glass account login did not match the maintenance window.', now() - interval '170 minutes', null),
  ('20000000-0000-4000-8000-000000000029', '10000000-0000-4000-8000-000000000033', :'responder_id', 'Patched dependency verified; advisory marked remediated.', now() - interval '40 hours', null),
  ('20000000-0000-4000-8000-000000000030', '10000000-0000-4000-8000-000000000034', :'responder_id', 'Rule 941100 moved to log-only pending a narrower expression.', now() - interval '480 minutes', null)
on conflict (id) do update set
  incident_id = excluded.incident_id, author_id = excluded.author_id,
  content = excluded.content, created_at = excluded.created_at,
  edited_at = excluded.edited_at;

insert into timeline_reactions (entry_id, user_id, emoji, created_at) values
  ('20000000-0000-4000-8000-000000000002', :'manager_id', '👍', now() - interval '11 minutes'),
  ('20000000-0000-4000-8000-000000000002', :'observer_id', '✅', now() - interval '10 minutes'),
  ('20000000-0000-4000-8000-000000000003', :'manager_id', '👀', now() - interval '5 minutes'),
  ('20000000-0000-4000-8000-000000000005', :'observer_id', '👍', now() - interval '35 minutes'),
  ('20000000-0000-4000-8000-000000000008', :'responder_id', '🚨', now() - interval '99 minutes'),
  ('20000000-0000-4000-8000-000000000017', :'observer_id', '🎉', now() - interval '44 hours'),
  ('20000000-0000-4000-8000-000000000022', :'responder_id', '✅', now() - interval '390 minutes'),
  ('20000000-0000-4000-8000-000000000027', :'observer_id', '👍', now() - interval '39 minutes')
on conflict (entry_id, user_id, emoji) do update set created_at = excluded.created_at;

-- E2E creation exercises the real Release form. Namespaced rows are removed so
-- the deterministic queue counts and related steps remain replayable.
delete from releases where title like 'E2E %';

insert into releases (id, team_id, title, base_state, created_at, updated_at) values
  ('30000000-0000-4000-8000-000000000001', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'v2.8.0 — Payment resilience', 'in_progress', now() - interval '1 day', now() - interval '20 hours'),
  ('30000000-0000-4000-8000-000000000002', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'v2.7.3 — Authentication hotfix', 'completed', now() - interval '4 days', now() - interval '92 hours'),
  ('30000000-0000-4000-8000-000000000003', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'v2.9.0 — Observability foundations', 'created', now() - interval '6 hours', now() - interval '6 hours'),
  ('30000000-0000-4000-8000-000000000004', '39aa8884-22cc-4764-a9e7-7df7c7619ba6', 'v2.6.1 — Legacy queue cleanup', 'cancelled', now() - interval '12 days', now() - interval '11 days'),
  ('30000000-0000-4000-8000-000000000005', '6d1e8c20-b622-4d21-9b1b-111111111111', 'eu-edge-2026.07.18', 'in_progress', now() - interval '8 hours', now() - interval '7 hours'),
  ('30000000-0000-4000-8000-000000000006', '6d1e8c20-b622-4d21-9b1b-111111111111', 'billing-workers-4.12.2', 'completed', now() - interval '3 days', now() - interval '67 hours'),
  ('30000000-0000-4000-8000-000000000007', '8b2f9d30-c733-4e32-8c2c-222222222222', 'iam-policy-hardening', 'in_progress', now() - interval '5 hours', now() - interval '4 hours'),
  ('30000000-0000-4000-8000-000000000008', '8b2f9d30-c733-4e32-8c2c-222222222222', 'waf-ruleset-31', 'created', now() - interval '10 hours', now() - interval '10 hours')
on conflict (id) do update set
  team_id = excluded.team_id, title = excluded.title,
  base_state = excluded.base_state, created_at = excluded.created_at,
  updated_at = excluded.updated_at;

insert into release_steps (release_id, position, name, validated_by, validated_at) values
  ('30000000-0000-4000-8000-000000000001', 0, 'Build and sign artifacts', :'manager_id', now() - interval '23 hours'),
  ('30000000-0000-4000-8000-000000000001', 1, 'Deploy to staging', :'responder_id', now() - interval '20 hours'),
  ('30000000-0000-4000-8000-000000000001', 2, 'Run payment smoke tests', null, null),
  ('30000000-0000-4000-8000-000000000001', 3, 'Promote to production', null, null),
  ('30000000-0000-4000-8000-000000000002', 0, 'Build hotfix', :'manager_id', now() - interval '95 hours'),
  ('30000000-0000-4000-8000-000000000002', 1, 'Validate SSO callbacks', :'responder_id', now() - interval '94 hours'),
  ('30000000-0000-4000-8000-000000000002', 2, 'Deploy globally', :'manager_id', now() - interval '92 hours'),
  ('30000000-0000-4000-8000-000000000003', 0, 'Publish dashboards', null, null),
  ('30000000-0000-4000-8000-000000000003', 1, 'Enable tracing sampler', null, null),
  ('30000000-0000-4000-8000-000000000003', 2, 'Validate alert routes', null, null),
  ('30000000-0000-4000-8000-000000000004', 0, 'Drain legacy workers', null, null),
  ('30000000-0000-4000-8000-000000000004', 1, 'Remove old queues', null, null),
  ('30000000-0000-4000-8000-000000000005', 0, 'Build edge image', :'responder_id', now() - interval '7 hours'),
  ('30000000-0000-4000-8000-000000000005', 1, 'Canary in Paris', null, null),
  ('30000000-0000-4000-8000-000000000005', 2, 'Roll out European POPs', null, null),
  ('30000000-0000-4000-8000-000000000006', 0, 'Build worker image', :'manager_id', now() - interval '70 hours'),
  ('30000000-0000-4000-8000-000000000006', 1, 'Replay billing sample', :'manager_id', now() - interval '69 hours'),
  ('30000000-0000-4000-8000-000000000006', 2, 'Deploy workers', :'responder_id', now() - interval '67 hours'),
  ('30000000-0000-4000-8000-000000000007', 0, 'Review IAM diff', :'observer_id', now() - interval '4 hours'),
  ('30000000-0000-4000-8000-000000000007', 1, 'Apply staging policy', null, null),
  ('30000000-0000-4000-8000-000000000007', 2, 'Apply production policy', null, null),
  ('30000000-0000-4000-8000-000000000008', 0, 'Run rule simulation', null, null),
  ('30000000-0000-4000-8000-000000000008', 1, 'Deploy in log-only mode', null, null),
  ('30000000-0000-4000-8000-000000000008', 2, 'Enable blocking mode', null, null)
on conflict (release_id, position) do update set
  name = excluded.name, validated_by = excluded.validated_by,
  validated_at = excluded.validated_at;

insert into release_incidents (release_id, incident_id) values
  ('30000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000001'),
  ('30000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000009'),
  ('30000000-0000-4000-8000-000000000005', '10000000-0000-4000-8000-000000000024'),
  ('30000000-0000-4000-8000-000000000007', '10000000-0000-4000-8000-000000000032')
on conflict (release_id, incident_id) do nothing;

-- Private-message browser tests exercise both HTTP directions and realtime
-- delivery. Keep only the stable conversation after each run.
delete from private_messages where content like 'E2E direct message %';

insert into private_messages (id, sender_id, recipient_id, content, created_at) values
  ('40000000-0000-4000-8000-000000000001', :'manager_id', :'responder_id', 'Can you take the checkout latency investigation?', now() - interval '50 minutes'),
  ('40000000-0000-4000-8000-000000000002', :'responder_id', :'manager_id', 'On it. I am checking the promotion lookup path first.', now() - interval '48 minutes'),
  ('40000000-0000-4000-8000-000000000003', :'observer_id', :'manager_id', 'Support has linked twelve customer reports to the payment incident.', now() - interval '16 minutes'),
  ('40000000-0000-4000-8000-000000000004', :'manager_id', :'observer_id', 'Thanks. Keep the incident channel updated with new regions.', now() - interval '14 minutes'),
  ('40000000-0000-4000-8000-000000000005', :'responder_id', :'observer_id', 'Could you validate whether the Android reports are all version 8.4.1?', now() - interval '6 hours'),
  ('40000000-0000-4000-8000-000000000006', :'observer_id', :'responder_id', 'Confirmed: 87% are 8.4.1, mostly Android 14.', now() - interval '350 minutes')
on conflict (id) do update set
  sender_id = excluded.sender_id, recipient_id = excluded.recipient_id,
  content = excluded.content, created_at = excluded.created_at;

insert into team_bans (team_id, user_id, expires_at, reason, created_by, created_at) values
  ('6d1e8c20-b622-4d21-9b1b-111111111111', :'contractor_id', null, 'Demo: access revoked after contract ended', :'responder_id', now() - interval '10 days'),
  ('8b2f9d30-c733-4e32-8c2c-222222222222', :'contractor_id', now() + interval '7 days', 'Demo: temporary security review', :'observer_id', now() - interval '1 day')
on conflict (team_id, user_id) do update set
  expires_at = excluded.expires_at, reason = excluded.reason,
  created_by = excluded.created_by, created_at = excluded.created_at;

commit;

select
  (select count(*) from teams) as teams,
  (select count(*) from team_members) as memberships,
  (select count(*) from incidents) as incidents,
  (select count(*) from timeline_entries) as timeline_entries,
  (select count(*) from timeline_reactions) as reactions,
  (select count(*) from releases) as releases,
  (select count(*) from release_steps) as release_steps,
  (select count(*) from release_incidents) as release_links,
  (select count(*) from private_messages) as private_messages,
  (select count(*) from team_bans) as bans;
