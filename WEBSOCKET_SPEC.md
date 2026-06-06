# WEBSOCKET_SPEC

Real-time channel for OpsWarden. One WebSocket connection per client. The server
pushes domain events to the connected members of a team; clients reconnect
automatically. This document is the contract between server and clients.

> Status: **Phase 1, PR1** — the four outbound incident/timeline events below are
> live. `presence_update` and the inbound `watch` command land in PR2; the
> extended events (`member_*`, `release_*`, reactions, private messages) come in
> later phases.

## Connection

- **Endpoint**: `GET /ws` (HTTP upgrade). The route is public.
- **Transport**: text frames carrying JSON objects, each with a `type` field.

## Authentication — first message

Browsers cannot set an `Authorization` header on the WS handshake, so the
connection authenticates **in-band**: the client's first frame must be

```json
{ "type": "auth", "token": "<jwt>" }
```

The server verifies the JWT (same `TokenService` as the REST API) and checks it
is not revoked (same blacklist as `POST /api/auth/logout`). On success the
connection is registered for every team the user belongs to. On any failure
(missing/invalid/expired/revoked token, malformed frame) the server closes the
connection. Until a valid `auth` frame is received, no events are delivered.

## Reconnection (client responsibility)

Automatic reconnection is **mandatory**. On disconnect the client reconnects with
backoff, re-sends the `auth` frame, and re-fetches current state over REST to
resynchronize (events received while disconnected are not replayed).

## Outbound events (server → client)

Delivered to every connected member of the event's team. `by` / `author` are user
ids; `at` is a Unix timestamp (seconds).

| `type` | Emitted when | Payload |
|---|---|---|
| `incident_state_changed` | an incident transitions state | `{ "type", "incident_id", "new_state", "by" }` |
| `incident_escalated` | an incident transitions to `escalated` | `{ "type", "incident_id", "new_severity", "by" }` |
| `incident_assigned` | a Manager assigns a responder | `{ "type", "incident_id", "assigned_to", "by" }` |
| `timeline_entry_added` | a timeline entry is posted | `{ "type", "incident_id", "entry": { "entry_id", "content", "author", "at" } }` |

Note: escalating an incident emits **both** `incident_state_changed`
(`new_state: "escalated"`) and `incident_escalated` (with the current severity).

### Examples

```json
{"type":"incident_state_changed","incident_id":"…","new_state":"acknowledged","by":"…"}
{"type":"incident_escalated","incident_id":"…","new_severity":"critical","by":"…"}
{"type":"incident_assigned","incident_id":"…","assigned_to":"…","by":"…"}
{"type":"timeline_entry_added","incident_id":"…","entry":{"entry_id":"…","content":"…","author":"…","at":1780000000}}
```

## Where it lives (architecture)

- **Events are born in the app layer**: each use case publishes a typed
  `domain::event::DomainEvent` through the `ports::EventPublisher` port after
  persisting its change. Publishing is fire-and-forget — a broadcast failure
  never fails the business operation.
- **Fan-out is an adapter**: `adapters/ws/hub.rs` (`WsHub`) implements
  `EventPublisher`, holds the connection registry, and sends to the connections
  whose user belongs to the event's team. The wire JSON is built in
  `adapters/ws/protocol.rs`.
- **Transport shell**: `handlers/ws.rs` performs the upgrade, the first-message
  auth, registers/unregisters the connection, and pumps events to the socket.
