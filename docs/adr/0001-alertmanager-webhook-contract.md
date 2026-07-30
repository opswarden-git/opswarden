# ADR 0001: Alertmanager webhook contract

- Status: Superseded by [ADR 0002](0002-alertmanager-lifecycle-contract.md)
- Date: 2026-07-30
- Scope: inbound Alertmanager notification groups

## Context

Alertmanager sends notification groups rather than a guaranteed one-request-per-
alert stream. OpsWarden needs an authenticated and idempotent first integration
without exposing arbitrary provider data to automation templates.

## Decision

### Transport and authentication

- The endpoint is `POST /webhooks/alertmanager/{connection_id}`.
- The content type must be JSON.
- The raw body is limited to 1 MiB.
- Authentication is `Authorization: Bearer <token>`.
- A Manager supplies the token when configuring the team connection. It is
  encrypted at rest and is never returned by the API.
- Verification compares SHA-256 digests in constant time.

### Validation and event semantics

- The body must be a JSON object with a top-level `status` of `firing` or
  `resolved` and an `alerts` array.
- A firing group must contain at least one alert.
- The top-level status is authoritative in `v1.0.11`.
- A firing group produces one `alert_firing` event for the complete group.
- A resolved group is authenticated and recorded as an ignored delivery. It
  does not produce an `alert_resolved` event.
- Per-alert status values are retained when they are strings, but they do not
  independently trigger or resolve rules.

### Idempotency

Alertmanager does not provide a delivery identifier. OpsWarden uses:

```text
sha256:<digest of the exact raw request body>
```

An exact retry is a duplicate. Any byte-level change, including semantically
equivalent JSON formatting, is a new delivery. This is safe and deterministic,
but a future transition-aware design may replace it.

### Normalization

The group event may contain:

- `status`, `alert_count`, `group_key` and `receiver`;
- shared `severity`, `alertname` and `summary` values;
- an `alerts` array containing only `status`, `fingerprint`, `alertname`,
  `severity` and `summary`.

Arbitrary labels, annotations and the raw body are unavailable to reaction
templates. Each normalized string is non-empty and at most 1,024 bytes.
Templates may use the scalar variables `severity`, `alertname`, `summary`,
`receiver` and `group_key` when present.

## Consequences

- Multiple alerts are not silently reduced to the first alert.
- Exact retries cannot create duplicate automation runs or incidents.
- Resolved notifications are observable in durable delivery state without
  changing incident state.
- A rule filtering `alertname` or using `{{alertname}}` only matches groups
  where the value is common to every alert.
- This contract is a foundation, not the final self-healing lifecycle.

## Follow-up decision

Before implementing automatic recovery, decide whether OpsWarden emits one event
per alert or one per group, add `alert_resolved` if required, define mixed-state
handling, and replace exact-body idempotency with a semantic transition key.
