# ADR 0004: Frontend composition contract

- Status: Accepted
- Date: 2026-08-25

## Context

OpsWarden accumulated several page compositions around stable server behavior.
The redesign must simplify navigation and presentation without rebuilding API
queries, RBAC, WebSocket delivery, Incident and Release lifecycles,
confirmations, translations or deep links.

Open-source references informed individual mechanics rather than a complete
product copy: DFIR-IRIS for investigation context, Mattermost for conversation
rooms, Keep and Grafana OnCall for scannable queues, and Primer for semantic
state and spacing systems.

## Decision

### One product grammar

- Collections expose one search/action rail. Desktop filtering and sorting live
  in table columns; mobile uses one sheet.
- Cards represent autonomous objects, never decorative wrappers.
- Members belongs to Team. Team switching belongs to the breadcrumb, not a
  second sidebar selector.
- Release remains a first-class workflow in product navigation, not an item in
  the conversation rail.
- Important information never depends only on color, hover or a pointer.

### Conversation rooms

War Room and direct messages share one technical and visual core: server-owned
capabilities, Room WebSocket identity, transcript, message bubble, composer,
presence and typing hooks. Incident and DM are adapters over that core.

The current author renders on the right, other authors on the left, and system
events use a neutral timeline grammar. Two consecutive human notes from the
same author may group within two minutes. Automation, system transitions,
escalation and a transition to critical severity always interrupt grouping.

The War Room is bounded: navigation, actions and composer remain available
while the transcript scrolls. The persistent Incident banner proposed in PR
#168 remains rejected because it duplicated Team, title, status and severity
and competed with the breadcrumb.

### Truthful asymmetry

| Capability             | Direct message | Incident War Room |
| ---------------------- | -------------- | ----------------- |
| Text, GIF and editing  | yes            | yes               |
| Reactions              | yes            | yes               |
| Presence and typing    | yes            | yes               |
| Private files          | yes            | no                |
| Cursor pagination      | yes            | no                |
| Collaborative pointers | no             | yes               |
| System events          | no             | yes               |

A capability moves between scopes only with a user need and a real server
contract. DM attachments remain authenticated, bounded, download-only,
`nosniff` and absent from history response bytes.

### Bounded delivered projections

The Overview Week/Month calendar projects the Incidents, Releases and Runs
already loaded by that page. It is not a durable Team audit journal.

War Room pointers are normalized, ephemeral, rate-limited, never persisted,
never echoed to their sender and delivered only to authorized co-watchers of
the same Incident. They are not global user tracking.

Relative ages retain their absolute value in `time[datetime]`. Release state
uses its own semantic token family rather than borrowing Incident meanings.

## Consequences

- A visual refactor preserves server contracts and tests unless a product
  decision explicitly changes them.
- EN/FR, role visibility, keyboard access and affected viewports are part of
  every transverse change.
- A Runs alias is not Activity. A durable Team journal, Loki access, runbooks,
  coding agents, broader presence, Incident tabs, severity rails or reconnect
  separators are not implied backlog.
- New product scope requires an observable objective, data and permission
  contracts, failure/audit behavior, acceptance tests and an explicit roadmap
  position.
