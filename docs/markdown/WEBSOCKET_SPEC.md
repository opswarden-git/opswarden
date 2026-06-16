# WEBSOCKET_SPEC

Real-time channel for OpsWarden. One WebSocket connection per client. The server
pushes domain events to the connected members of a team; clients reconnect
automatically. This document is the contract between server and clients.

> Status: **Phase 1-2 live** — the four incident/timeline events, `presence_update`,
> the inbound `watch`/`unwatch`/`status_typing` commands with their `user_typing`
> broadcast, and the Phase 2 automation events `rule_triggered`/`rule_failed` are all
> implemented. The extended events (`member_*`, `release_*`, reactions, private
> messages) come in later phases.

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

## Inbound commands (client → server)

After authenticating, a client may send commands. Unknown or malformed frames are
ignored (forward-compatible). Presence and typing are **ephemeral**: never
persisted, forgotten when the connection closes.

| `type`          | Effect                                                                            | Payload                                           |
| --------------- | --------------------------------------------------------------------------------- | ------------------------------------------------- |
| `watch`         | start watching an incident; co-watchers receive a `presence_update`               | `{ "type": "watch", "incident_id": "…" }`         |
| `unwatch`       | stop watching an incident; remaining co-watchers receive a `presence_update`      | `{ "type": "unwatch", "incident_id": "…" }`       |
| `status_typing` | signal the user is typing on an incident; its co-watchers receive a `user_typing` | `{ "type": "status_typing", "incident_id": "…" }` |

A client typically sends `watch` when an incident view opens and `unwatch` when it
closes. Disconnecting is an implicit `unwatch` of everything.

## Outbound events (server → client)

Incident/timeline events are delivered to every connected member of the event's
team. `presence_update` and `user_typing` are delivered only to the **co-watchers**
of the incident (the clients currently watching it). `by` / `author` are user ids;
`at` is a Unix timestamp (seconds).

| `type`                   | Emitted when                                  | Payload                                                                         |
| ------------------------ | --------------------------------------------- | ------------------------------------------------------------------------------- |
| `incident_state_changed` | an incident transitions state                 | `{ "type", "incident_id", "new_state", "by" }`                                  |
| `incident_escalated`     | an incident transitions to `escalated`        | `{ "type", "incident_id", "new_severity", "by" }`                               |
| `incident_assigned`      | a Manager assigns a responder                 | `{ "type", "incident_id", "assigned_to", "by" }`                                |
| `timeline_entry_added`   | a timeline entry is posted                    | `{ "type", "incident_id", "entry": { "entry_id", "content", "author", "at" } }` |
| `presence_update`        | the watcher set of an incident changes        | `{ "type", "incident_id", "watchers": ["…"] }`                                  |
| `user_typing`            | a co-watcher signals it is typing             | `{ "type", "incident_id", "user_id" }`                                          |
| `rule_triggered`         | a Phase 2 automation rule reacted to an event | `{ "type", "service", "rule", "incident_id" }`                                  |
| `rule_failed`            | a Phase 2 automation rule's reaction failed   | `{ "type", "service", "rule", "reason" }`                                       |

Note: escalating an incident emits **both** `incident_state_changed`
(`new_state: "escalated"`) and `incident_escalated` (with the current severity).
`watchers` is the list of **distinct** user ids watching the incident (a user with
two tabs appears once).

### Examples

```json
{"type":"incident_state_changed","incident_id":"…","new_state":"acknowledged","by":"…"}
{"type":"incident_escalated","incident_id":"…","new_severity":"critical","by":"…"}
{"type":"incident_assigned","incident_id":"…","assigned_to":"…","by":"…"}
{"type":"timeline_entry_added","incident_id":"…","entry":{"entry_id":"…","content":"…","author":"…","at":1780000000}}
{"type":"presence_update","incident_id":"…","watchers":["…","…"]}
{"type":"user_typing","incident_id":"…","user_id":"…"}
{"type":"rule_triggered","service":"github","rule":"…","incident_id":"…"}
{"type":"rule_failed","service":"github","rule":"…","reason":"…"}
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
- **Presence and typing are ephemeral transport state**: who watches / is typing on
  which incident lives in the `WsHub` (never the domain, never the database).
  `watch`/`unwatch` mutate the per-connection watch set and broadcast a
  `presence_update`; `status_typing` broadcasts a `user_typing` to the co-watchers.
- **Transport shell**: `handlers/ws.rs` performs the upgrade, the first-message
  auth, registers/unregisters the connection, dispatches inbound commands to the
  hub, and pumps events to the socket.
