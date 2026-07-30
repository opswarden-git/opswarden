# Production, Alertmanager and quality roadmap

This roadmap records the post-recovery state on `main`. “Shipped” requires a
merge to `main` and a green required gate. Production-only evidence is not
considered reproducible until its corresponding Ops change is also merged.

## Current state

| Area                            | Status  | Evidence                                                                                                        | Remaining work                                                              |
| ------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Recovery and corrective release | Shipped | PR #122 was reverted; `v1.0.10` restored the app and `v1.0.11` reintroduced the safe Alertmanager foundation    | Preserve history for audit                                                  |
| Protected delivery              | Shipped | Pull requests, current required gate, linear history, admin enforcement, no force-push/delete                   | Require human approval when a second maintainer is available                |
| Source hygiene                  | Shipped | PRs #131–#134; no tracked owned source file exceeds 500 lines                                                   | Keep the 500-line ratchet                                                   |
| Lifecycle contract              | Shipped | [ADR 0002](adr/0002-alertmanager-lifecycle-contract.md), per-alert firing/resolved events and mixed-state tests | Observe real traffic                                                        |
| Semantic idempotency            | Shipped | Versioned transition identity and formatting/lifecycle retry tests                                              | Tune duplicate-ratio alerts from real traffic                               |
| HTTP/security matrix            | Shipped | 1 MiB, authentication, provider mismatch, disabled rules, tenant isolation and persisted reaction failure tests | Add mutation tests                                                          |
| Metrics                         | Shipped | Five counters, production Prometheus scrape, Grafana dashboard and three alert rules                            | Tune thresholds from real traffic                                           |
| Real Alertmanager E2E           | Shipped | CI plus production firing/resolved, two durable runs, `accepted +2`, `failed +0`                                | None                                                                        |
| Frontend onboarding             | Partial | English/French catalog, icon, generic accessibility contract and component test                                 | Add focused Alertmanager accessibility/visual assertions and lifecycle copy |
| Operator documentation          | Shipped | Setup, contract, metrics, rotation, troubleshooting and deployed rollback proofs                                | Keep operational evidence current                                           |
| Structural refactoring          | Shipped | All 18 grandfathered files were split by behavior-neutral PRs #131–#134                                         | Keep future files below 500 lines                                           |
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
  migration delta;
- restricted FRA1 Spaces credentials encrypted with SOPS/age;
- real encrypted backup upload with three downloaded matching objects;
- checksum and full isolated PostgreSQL restore with schema and SQLx migration
  assertions.

## Remaining work, in priority order

### P0 — Publish the proven Ops state

The backup resources and documentation are already active and proven in
production, but their `opswarden-ops` worktree still needs a reviewed PR, green
Ops CI and merge. No application release is required for this infrastructure
change.

### P1 — Backup operations

1. Observe the first scheduled `02:17 UTC` execution.
2. Alert on CronJob failure and stale backup age.
3. Prove retention deletion after 30 days.
4. Store the matching age identity in an audited offline recovery location.
5. Decide whether daily full-download verification is worth Cold Storage
   retrieval charges.
6. Monitor the FRA1 internal Spaces `/32` allowlist for DNS changes.

### P2 — Frontend acceptance

1. Add an accessibility assertion for the Alertmanager connection and both
   lifecycle actions.
2. Add a focused visual/browser onboarding assertion.
3. Explain `alert_resolved` in rule creation copy so it is not mistaken for an
   automatic incident close.

### P3 — Test effectiveness

- Introduce targeted mutation testing for authentication, provider mismatch,
  tenant isolation, semantic identity, filters and lifecycle status.
- Keep given/when/then names and assertions on durable effects.
- Keep the required gate duration bounded.

### P4 — Remaining production hardening

1. Version ConfigMaps, HPA and PDB as one atomic release unit.
2. Formalize forward-only migration recovery boundaries and rehearse full
   disaster recovery including credential rotation.
3. Add centralized log alerting.
4. Pin mutable application build-stage base images.
5. Add an explicit WebSocket Origin allowlist for the Vercel frontend.
6. Tune Alertmanager rejection and duplicate thresholds from real traffic.

## Release rule

Prepare versions in a dedicated pull request. After its required gate is green:

1. merge the release PR into `main`;
2. fetch and verify the resulting `main` commit;
3. create the annotated tag on that merged commit;
4. push the tag and monitor artifact publication.

Never tag a release branch before merge. Squash merging changes the commit ID,
and release automation rejects tags that are not reachable from `main`.
