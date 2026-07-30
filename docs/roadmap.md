# Recovery and Alertmanager roadmap

This document tracks the recovery plan against the code and releases that
actually shipped. The baseline is `v1.0.11`, published on 2026-07-30.

Status meanings:

- **Complete**: shipped on `main` and covered by the required CI gate.
- **Partial**: the safe foundation shipped, but acceptance work remains.
- **Open**: not implemented yet.

## Current state

| Area                                | Status   | Shipped in `v1.0.11`                                                                                                                                                                                 | Still required                                                                                                                                            |
| ----------------------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Recovery and corrective release     | Complete | PR #122 was reverted, `v1.0.10` restored the known application, and `v1.0.11` reintroduced Alertmanager from that clean baseline.                                                                    | Nothing for recovery. Keep the historical commits for audit purposes.                                                                                     |
| Protected delivery                  | Complete | `main` requires a current `CI · Required gate`, pull requests, resolved conversations and linear history. Admin enforcement is enabled; force-pushes and deletion are disabled.                      | Require one human approval when a second regular maintainer is available.                                                                                 |
| Source hygiene                      | Complete | New source and test files are capped at 500 lines; oversized legacy files cannot grow; unsafe TypeScript bypasses and unrelated test deletion fail CI.                                               | Continue shrinking grandfathered files toward the 200–350 line target.                                                                                    |
| Catalog contract                    | Complete | The health contract names all eight services explicitly and Alertmanager has a dedicated catalog module with localized labels and filters.                                                           | Continue moving the remaining provider definitions out of the 730-line catalog root.                                                                      |
| Alertmanager backend foundation     | Partial  | A 1 MiB JSON endpoint, encrypted bearer credential, constant-time token verification, exact-retry deduplication, allowlisted normalization, durable receipt and focused unit/HTTP tests are shipped. | Add the missing security/error matrix, semantic transition idempotency, dedicated metrics and a decision on resolved events.                              |
| Alertmanager frontend foundation    | Partial  | Alertmanager was added without removing GitHub, GitLab, Kubernetes, Sentry, Datadog or PagerDuty. English/French copy, an icon and a typed list are covered by component tests.                      | Add an accessibility assertion and a focused visual or browser-level onboarding test.                                                                     |
| Alertmanager operator documentation | Complete | The supported contract, setup, token rotation and troubleshooting are documented in [Alertmanager integration](integrations/alertmanager.md).                                                        | Keep the example synchronized with the upstream Alertmanager configuration schema.                                                                        |
| Alertmanager user-level validation  | Open     | The generic release E2E remains green.                                                                                                                                                               | Add a browser/API critical path that configures a connection and rule, fires twice, verifies one incident, then exercises the chosen resolution behavior. |
| Structural refactoring              | Open     | The ratchet prevents further growth.                                                                                                                                                                 | Split the seven grandfathered files listed below in behavior-neutral PRs.                                                                                 |
| Mutation testing                    | Open     | Existing unit, integration, coverage and E2E gates are green.                                                                                                                                        | Add targeted mutation testing for authentication, tenant isolation, idempotency, filtering, incident creation and firing/resolved handling.               |
| Alertmanager release                | Complete | `v1.0.11` passed quality, backend/web tests, release E2E, native packaging, container builds and provenance attestation.                                                                             | Run and record a smoke test against a real Alertmanager deployment and observe webhook outcomes after rollout.                                            |

## The `v1.0.11` contract

The current behavior is intentional and documented in
[ADR 0001](adr/0001-alertmanager-webhook-contract.md):

- one Alertmanager notification group produces at most one `alert_firing`
  event;
- a top-level `resolved` notification is authenticated, deduplicated and
  stored as ignored; it does not emit `alert_resolved`;
- the idempotency key is the SHA-256 digest of the exact raw request body;
- arbitrary labels and annotations never become automation variables.

These choices are safe for the first release, but they are not yet the complete
self-healing contract.

## Remaining work, in priority order

### P0 — Operational confidence

1. Run a real Alertmanager smoke test in a non-production environment.
2. Add counters or traces for accepted, rejected, duplicate, ignored and failed
   Alertmanager deliveries.
3. Verify token rotation and rollback in the deployed environment.
4. Record the smoke-test evidence and observed payload in a short operations
   report without credentials.

### P1 — Complete the event contract

Decide and document:

- whether a group remains one event or becomes one event per alert;
- whether `resolved` emits `alert_resolved`;
- how mixed per-alert states are handled;
- which transition fields form semantic idempotency
  (`connection_id`, `groupKey`, receiver, status, fingerprints and timestamps).

Write the unit tests first, then change the parser and catalog. Exact-body
deduplication must remain until the replacement can prove that retries are
ignored while real transitions are accepted.

### P1 — Close the HTTP/security test matrix

Add focused Alertmanager cases for:

- body above 1 MiB;
- unknown connection and a connection for another provider;
- disabled rule and non-matching filters;
- team isolation;
- failed reaction persisted as a failed automation run;
- non-text normalized fields and oversized normalized strings;
- changed payload accepted as a new delivery;
- the final mixed/resolved semantics selected above.

Keep these tests in dedicated files rather than growing
`team_webhooks.rs`.

### P2 — User-level E2E

Add the critical path:

1. configure an Alertmanager bearer token;
2. create and enable a filtered rule;
3. send a firing group;
4. verify the incident and automation run;
5. retry the identical body and verify no duplicate;
6. send the chosen resolution transition and verify its durable effect.

### P2 — Shrink the grandfathered files

Each refactor must be behavior-neutral and independently reviewable.

| File                                              | Lines in `v1.0.11` | Suggested split                                                              |
| ------------------------------------------------- | -----------------: | ---------------------------------------------------------------------------- |
| `server/tests/team_webhooks.rs`                   |              1,587 | provider delivery, authentication, idempotency, tenancy, limits and fixtures |
| `server/tests/team_automation.rs`                 |              1,501 | connections, rules, authorization, filters, templates and reactions          |
| `server/tests/common/mod.rs`                      |              1,519 | fixtures, builders, repositories, test context, authentication and requests  |
| `server/src/domain/automation_catalog.rs`         |                730 | provider and reaction modules                                                |
| `server/src/handlers/mod.rs`                      |                587 | API conversion, localization and shared mapping                              |
| `server/src/adapters/pg/automation/timer.rs`      |                904 | queries, acquisition, leases, row mapping and tests                          |
| `client-web/components/automations/RulesView.tsx` |                638 | trigger/reaction forms, filters, validation, hooks and presentation          |

### P3 — Test effectiveness

- Document and apply `given / when / then` for new behavior tests.
- Introduce targeted mutation tests rather than enabling mutation testing for
  the whole repository at once.
- Make failures actionable and keep the required gate duration bounded.

## Release rule

A release version is prepared in a PR. Only after that PR is merged may the
annotated tag be created on the resulting `main` commit. Never push a release
tag that points to the release branch: the release workflow deliberately rejects
tags that are not reachable from `main`.
