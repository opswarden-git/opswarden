# Recovery, Alertmanager and quality roadmap

This roadmap distinguishes the `v1.0.11` recovery baseline from the lifecycle
work implemented after it. “Implemented” means present on the feature branch;
“shipped” requires merge to `main` and a green required gate.

## Current state

| Area                            | Status  | Evidence                                                                                                        | Remaining work                                                              |
| ------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Recovery and corrective release | Shipped | PR #122 was reverted; `v1.0.10` restored the app and `v1.0.11` reintroduced the safe Alertmanager foundation    | Preserve history for audit                                                  |
| Protected delivery              | Shipped | Pull requests, current required gate, linear history, admin enforcement, no force-push/delete                   | Require human approval when a second maintainer is available                |
| Source hygiene                  | Shipped | New files capped at 500 lines; oversized legacy files cannot grow                                               | Split grandfathered files below                                             |
| Lifecycle contract              | Shipped | [ADR 0002](adr/0002-alertmanager-lifecycle-contract.md), per-alert firing/resolved events and mixed-state tests | Observe real traffic                                                        |
| Semantic idempotency            | Shipped | Versioned transition identity and formatting/lifecycle retry tests                                              | Tune duplicate-ratio alerts from real traffic                               |
| HTTP/security matrix            | Shipped | 1 MiB, authentication, provider mismatch, disabled rules, tenant isolation and persisted reaction failure tests | Add mutation tests                                                          |
| Metrics                         | Shipped | Five counters, production Prometheus scrape, Grafana dashboard and three alert rules                            | Tune thresholds from real traffic                                           |
| Real Alertmanager E2E           | Shipped | CI plus production firing/resolved, two durable runs, `accepted +2`, `failed +0`                                | None                                                                        |
| Frontend onboarding             | Partial | English/French catalog, icon and typed list covered by component tests                                          | Add focused accessibility and visual onboarding assertions                  |
| Operator documentation          | Shipped | Setup, contract, metrics, rotation, troubleshooting and deployed rollback proofs                                | Keep operational evidence current                                           |
| Structural refactoring          | Open    | The ratchet prevents growth                                                                                     | Split oversized files in behavior-neutral PRs                               |
| Mutation testing                | Open    | Unit, integration and E2E coverage exists                                                                       | Target authentication, tenancy, identity, filtering and lifecycle mutations |

## Final self-healing contract

The decisions are recorded in
[ADR 0002](adr/0002-alertmanager-lifecycle-contract.md):

- one event per alert, never one event for the whole group;
- the per-alert status is authoritative, including mixed groups;
- `firing` emits `alert_firing` and `resolved` emits `alert_resolved`;
- semantic identity uses the connection repository key plus `groupKey`,
  receiver, fingerprint, status and `startsAt`; resolved also uses `endsAt`;
- formatting, labels, annotations and firing `endsAt` changes are retries, while
  real lifecycle or start-time changes are new transitions.

OpsWarden emits durable lifecycle events. Automatically resolving or mutating
an earlier incident is not implicit; teams choose reactions for
`alert_resolved`.

## Operational closure delivered

The release and operational tasks formerly listed as P0 are complete:

- release `v1.0.12` from `main`, with green CI, release E2E and immutable
  artifacts;
- production Prometheus scrape, dashboard and alert rules;
- credential-free production firing/resolved report;
- token rotation and rollback with stale-secret rejection;
- Metrics Server, populated HPA metrics and a controlled `2 → 3 → 2` proof;
- production NetworkPolicies with positive and negative connectivity checks;
- immutable `v1.0.12 → v1.0.11 → v1.0.12` rollback proof without a SQL
  migration delta.

The off-cluster PostgreSQL backup proof still requires externally provisioned,
restricted DigitalOcean Spaces credentials. No such credentials are currently
available to CI or the local workspace.

## Remaining work, in priority order

### P1 — Frontend acceptance

1. Add an accessibility assertion for the Alertmanager connection and both
   lifecycle actions.
2. Add a focused visual/browser onboarding assertion.
3. Explain `alert_resolved` in rule creation copy so it is not mistaken for an
   automatic incident close.

### P2 — Shrink grandfathered files

Each refactor must be behavior-neutral and independently reviewable.

| File                                                      | Current lines | Suggested split                                                  |
| --------------------------------------------------------- | ------------: | ---------------------------------------------------------------- |
| `server/tests/team_webhooks.rs`                           |         1,587 | providers, authentication, identity, tenancy, limits, fixtures   |
| `server/tests/common/mod.rs`                              |         1,519 | context, builders, repositories, authentication, requests        |
| `server/tests/team_automation.rs`                         |         1,501 | connections, rules, authorization, filters, templates, reactions |
| `server/src/adapters/pg/automation/timer.rs`              |           904 | queries, acquisition, leases, row mapping, tests                 |
| `server/tests/incidents.rs`                               |           829 | lifecycle, RBAC, timeline, realtime                              |
| `server/src/domain/automation_catalog.rs`                 |           730 | provider triggers, reactions and catalog assembly                |
| `server/src/adapters/pg/automation/service_connection.rs` |           729 | writes, reads, OAuth vault mapping and row conversion            |
| `server/src/handlers/team_automation.rs`                  |           727 | connections, rules, runs and OAuth handlers                      |
| `server/src/adapters/pg/team.rs`                          |           700 | membership, invitations, moderation and row mapping              |
| `server/src/handlers/incident.rs`                         |           640 | commands, queries, timeline and response mapping                 |
| `client-web/components/automations/RulesView.tsx`         |           638 | forms, filters, validation, hooks and presentation               |
| `server/src/domain/automation_config.rs`                  |           637 | trigger configs, reaction configs and validation                 |
| `server/tests/teams.rs`                                   |           625 | membership, invitations, moderation and deletion                 |
| `server/src/handlers/mod.rs`                              |           587 | API conversion, localization and shared mapping                  |
| `server/src/app/automation/reaction_executor.rs`          |           581 | reaction dispatch and provider-specific execution                |
| `server/src/adapters/ws/protocol.rs`                      |           564 | message types, parsing and protocol tests                        |
| `server/src/ports/mod.rs`                                 |           557 | repository ports grouped by bounded context                      |
| `server/src/adapters/pg/incident.rs`                      |           519 | queries, mutations and row mapping                               |

### P3 — Test effectiveness

- Introduce targeted mutation testing for authentication, provider mismatch,
  tenant isolation, semantic identity, filters and lifecycle status.
- Keep given/when/then names and assertions on durable effects.
- Keep the required gate duration bounded.

## Release rule

Prepare versions in a dedicated pull request. After its required gate is green:

1. merge the release PR into `main`;
2. fetch and verify the resulting `main` commit;
3. create the annotated tag on that merged commit;
4. push the tag and monitor artifact publication.

Never tag a release branch before merge. Squash merging changes the commit ID,
and release automation rejects tags that are not reachable from `main`.
