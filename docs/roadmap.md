# Production, Alertmanager and quality roadmap

This roadmap records the post-recovery state on `main`. “Shipped” requires a
merge to `main` and a green required gate. Production-only evidence is not
considered reproducible until its corresponding Ops change is also merged.

## Current state

| Area                            | Status  | Evidence                                                                                                        | Remaining work                                                       |
| ------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Recovery and corrective release | Shipped | PR #122 was reverted; `v1.0.10` restored the app and `v1.0.11` reintroduced the safe Alertmanager foundation    | Preserve history for audit                                           |
| Protected delivery              | Shipped | Pull requests, current required gate, linear history, admin enforcement, no force-push/delete                   | Require human approval when a second maintainer is available         |
| Source hygiene                  | Shipped | PRs #131–#134; no tracked owned source file exceeds 500 lines                                                   | Keep the 500-line ratchet                                            |
| Lifecycle contract              | Shipped | [ADR 0002](adr/0002-alertmanager-lifecycle-contract.md), per-alert firing/resolved events and mixed-state tests | Observe real traffic                                                 |
| Semantic idempotency            | Shipped | Versioned transition identity and formatting/lifecycle retry tests                                              | Tune duplicate-ratio alerts from real traffic                        |
| HTTP/security matrix            | Shipped | 1 MiB, authentication, provider mismatch, disabled rules, tenant isolation and persisted reaction failure tests | None                                                                 |
| Metrics                         | Shipped | Five counters, production Prometheus scrape, Grafana dashboard and three alert rules                            | Tune thresholds from real traffic                                    |
| Real Alertmanager E2E           | Shipped | CI plus production firing/resolved, two durable runs, `accepted +2`, `failed +0`                                | None                                                                 |
| Frontend onboarding             | Shipped | English/French lifecycle copy, focused component acceptance and mobile browser assertion                        | None                                                                 |
| Operator documentation          | Shipped | Setup, contract, metrics, rotation, troubleshooting and deployed rollback proofs                                | Keep operational evidence current                                    |
| Structural refactoring          | Shipped | All 18 grandfathered files were split by behavior-neutral PRs #131–#134                                         | Keep future files below 500 lines                                    |
| Mutation testing                | Shipped | Pinned bounded campaign: 48 mutants, 45 caught, three compile-time unviable, zero survivors/timeouts            | Expand only when a new critical boundary justifies the gate duration |
| Backup operations               | Shipped | Encrypted upload/restore plus five healthy CronJob freshness/failure rules in production                        | Retain the time-gated scheduled and 30-day deletion evidence         |
| Release-state recovery          | Shipped | Reverse-order snapshot restoration, release annotations and production disposable-resource proof                | Do not inject a full failed production release solely for evidence   |
| Centralized logs                | Shipped | Digest-pinned Loki/Alloy, retained PVC, Grafana source, two healthy alerts and queried production marker        | Decide whether multi-zone object storage is warranted                |
| Supply-chain/WS/migrations      | Shipped | Digest-only Docker bases, exact Origin allowlist and immutable phase-labelled migration CI                      | Keep the ratchets enabled                                            |

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
  migration delta;
- restricted FRA1 Spaces credentials encrypted with SOPS/age;
- real encrypted backup upload with three downloaded matching objects;
- checksum and full isolated PostgreSQL restore with schema and SQLx migration
  assertions.

## P0–P3 closure

The recovery roadmap is closed:

- **P0:** backup manifests, encrypted secret and restore evidence are merged;
- **P1:** CronJob state, failure and 26-hour freshness are monitored in
  production;
- **P2:** both Alertmanager lifecycle actions have focused accessible component
  and browser acceptance;
- **P3:** targeted mutation testing has no surviving viable mutant;
- **production hardening:** Kubernetes release state is restorable, migrations
  are forward-only, logs are centralized, Docker bases are immutable and
  browser WebSocket origins are allow-listed.

## Remaining operational evidence

These are time- or operator-gated observations, not missing implementation:

1. retain the first scheduled `02:17 UTC` backup proof;
2. retain evidence of deletion after the real 30-day retention window;
3. place the matching age identity in an audited offline recovery location;
4. reevaluate full-download Cold Storage verification costs and the FRA1 DNS
   `/32`;
5. tune Alertmanager thresholds from real traffic;
6. decide whether seven-day retained-volume logs also need multi-zone object
   storage.

## Release rule

Prepare versions in a dedicated pull request. After its required gate is green:

1. merge the release PR into `main`;
2. fetch and verify the resulting `main` commit;
3. create the annotated tag on that merged commit;
4. push the tag and monitor artifact publication.

Never tag a release branch before merge. Squash merging changes the commit ID,
and release automation rejects tags that are not reachable from `main`.
