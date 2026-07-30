# ADR 0002: Alertmanager lifecycle contract

- Status: Accepted
- Date: 2026-07-30
- Supersedes: [ADR 0001](0001-alertmanager-webhook-contract.md)

## Context

The `v1.0.11` foundation treated a notification group as one firing event,
ignored resolved groups and deduplicated exact request bytes. That was safe for
initial delivery but insufficient for self-healing: a group can contain several
alerts with different states, and harmless JSON or `endsAt` changes can occur on
retries.

## Decision

### Lifecycle unit

Each item in `alerts` is one independent lifecycle transition. Its own `status`
is authoritative:

- `firing` emits `alert_firing`;
- `resolved` emits `alert_resolved`;
- a mixed notification group emits both kinds as appropriate.

The top-level status is validated but never overrides an alert's status.
Notifications with no alerts are rejected because they contain no identifiable
transition.

### Semantic idempotency

The delivery identity is a versioned SHA-256 digest over:

```text
groupKey, receiver, fingerprint, status, startsAt,
endsAt when status is resolved
```

The connection ID remains part of the durable repository key. JSON ordering,
whitespace, annotations, labels and firing `endsAt` changes do not create a new
transition. A new `startsAt`, a firing-to-resolved change, or a different
resolved `endsAt` is a new transition. Duplicate identities inside one request
are rejected as ambiguous.

### Validation and normalized data

The transport remains JSON with a 1 MiB limit and a team-scoped bearer token.
Every alert requires `status`, `fingerprint` and `startsAt`; resolved alerts also
require `endsAt`. Labels and annotations must be objects when present.

Only these bounded scalar values reach filters and templates:

- lifecycle: `status`, `fingerprint`, `starts_at`, `ends_at`, `generator_url`;
- routing: `group_key`, `receiver`;
- labels: `alertname`, `severity`, `instance`, `namespace`, `pod`, `service`,
  `job`;
- annotations: `summary`, `description`.

Unknown, non-text, empty or values above 1,024 bytes are not normalized.
Credentials and raw payloads are never persisted as automation attributes.

### Observability

`GET /metrics` exposes one counter per alert transition outcome:
`accepted`, `rejected`, `duplicate`, `ignored` and `failed`. A transition is
ignored when it is valid but matches no enabled rule. A reaction failure is
persisted in the automation run and counted as failed.

## Consequences

- One noisy group can create several runs; rule filters should remain specific.
- Firing and recovery reactions can be configured independently.
- Formatting-only retries cannot duplicate a reaction.
- OpsWarden records lifecycle events but does not yet decide how a resolved
  event mutates an existing incident; that remains a rule/reaction concern.
