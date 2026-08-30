# WebSocket protocol

This is the canonical wire contract shared by the Rust server, Next.js client
and Tauri client. Business mutations use REST; WebSocket frames notify connected
clients after successful mutations and carry ephemeral collaboration state.

## Transport and lifecycle

- Endpoint: `GET /ws`, upgraded to WebSocket.
- Frames are UTF-8 JSON objects with a mandatory string `type`.
- UUIDs are strings; dates are Unix timestamps in seconds; `null` means no
  expiry or no resulting resource.
- The server ignores malformed or unknown commands after authentication and
  never persists or replays events.
- Each client uses one connection and reconnects automatically.

Browser handshakes require exactly one `Origin` matching
`OPSWARDEN_WS_ALLOWED_ORIGINS`; invalid, cross-site, `null` or repeated origins
receive HTTP 403. Originless native/service clients remain allowed but must
authenticate in-band.

The first text frame must be:

```json
{ "type": "auth", "token": "<access-token>" }
```

The server closes on a missing, malformed, invalid or revoked token. Ping, pong
and binary frames may precede authentication; another text frame may not. After
authentication, the connection is scoped to the user's current Teams. On every
reconnection, the client authenticates first, invalidates/refetches active REST
projections and replays its active room watches because missed events are not
replayed.

## Client commands

| Command                   | Exact fields after `type` | Acceptance and effect                                                                                         |
| ------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `watch`                   | `incident_id`             | Incident exists in a current Team; joins its room and emits `presence_update`.                                |
| `unwatch`                 | `incident_id`             | Removes this connection from the room and updates remaining watchers.                                         |
| `status_typing`           | `incident_id`             | Current Team role has `can_signal_typing` and connection watches the Incident; emits ephemeral `user_typing`. |
| `cursor`                  | `incident_id`, `x`, `y`   | Connection watches the Incident and finite coordinates are in `0…1`; relays to other watchers only.           |
| `refresh_teams`           | none                      | Reloads memberships from PostgreSQL and replaces routing/authorization scope.                                 |
| `watch_private_message`   | `peer_id`                 | Distinct users currently share a Team; joins their normalized bilateral room.                                 |
| `unwatch_private_message` | `peer_id`                 | Leaves the normalized bilateral room.                                                                         |
| `private_message_typing`  | `peer_id`                 | Distinct users share a Team and sender watches the room; relays ephemerally to other room watchers.           |

The client sends `refresh_teams` after Team create, join, leave, kick, ban or
delete operations that can invalidate the cached membership scope.

## Delivery scopes

- **Team:** every live connection whose cached membership contains `team_id`.
- **Room:** connections actively watching the Incident or normalized bilateral
  room; “other watchers” excludes the sending connection.
- **Users:** every live connection owned by an explicitly listed user.

Business actions enforce permissions before publishing. Presence, typing and
cursor commands are authorized again at the WebSocket edge. Private-message
content is never broadcast to an entire Team.

## Server events

Every table row defines the exact fields after `type`, its emission condition
and its recipients. Routing-only domain fields are not serialized unless named.

### Incidents and timelines

| Event                    | Exact payload fields                                      | Emission condition                                | Recipients |
| ------------------------ | --------------------------------------------------------- | ------------------------------------------------- | ---------- |
| `incident_created`       | `incident_id`, `severity`                                 | Incident persisted, including Automation creation | Team       |
| `incident_state_changed` | `incident_id`, `new_state`, `by`                          | Authorized lifecycle transition persisted         | Team       |
| `incident_escalated`     | `incident_id`, `new_severity`, `by`                       | Authorized severity escalation persisted          | Team       |
| `incident_assigned`      | `incident_id`, `assigned_to`, `by`                        | Responder assignment persisted                    | Team       |
| `timeline_entry_added`   | `incident_id`, `entry: { entry_id, content, author, at }` | Timeline entry persisted                          | Team       |
| `timeline_entry_edited`  | `incident_id`, `entry_id`, `new_content`, `edited_at`     | Author edit persisted                             | Team       |
| `reaction_added`         | `incident_id`, `entry_id`, `emoji`, `by`                  | Incident timeline reaction persisted              | Team       |
| `reaction_removed`       | `incident_id`, `entry_id`, `emoji`, `by`                  | Caller's Incident timeline reaction removed       | Team       |

Reactions exist only on Incident timeline entries. Release validations and
private messages do not expose reaction commands or events.

### Presence and collaboration

| Event                      | Exact payload fields                       | Emission condition                                   | Recipients                    |
| -------------------------- | ------------------------------------------ | ---------------------------------------------------- | ----------------------------- |
| `presence_update`          | `resource_id`, `resource_type`, `watchers` | Incident watch, unwatch or watched disconnect        | Incident room watchers        |
| `team_presence_update`     | `team_id`, `online_user_ids`               | Connection register/unregister or Team-scope refresh | Team                          |
| `user_typing`              | `incident_id`, `user_id`                   | Authorized `status_typing` accepted                  | Other Incident watchers       |
| `cursor_update`            | `incident_id`, `user_id`, `x`, `y`         | Authorized normalized cursor accepted                | Other Incident watchers       |
| `private_message_presence` | `participants`, `watchers`                 | Bilateral watch, unwatch or watched disconnect       | Bilateral room watchers       |
| `private_message_typing`   | `from`, `to`                               | Authorized bilateral typing accepted                 | Other bilateral room watchers |

`resource_type` is currently the literal `incident`. Presence arrays contain
distinct user IDs even when one user has several connections. Typing and cursor
state expire client-side and are never persisted.

### Team moderation

| Event           | Exact payload fields               | Emission condition                                | Recipients |
| --------------- | ---------------------------------- | ------------------------------------------------- | ---------- |
| `member_kicked` | `team_id`, `member`, `by`          | Membership removed and assignments cleared        | Team       |
| `member_banned` | `team_id`, `member`, `until`, `by` | Temporary, permanent or pre-emptive ban persisted | Team       |

`until` is the expiry timestamp for a temporary ban and `null` for a permanent
ban. A removed member can receive the final event through its cached connection
scope, then refreshes membership.

### Private messages

| Event                      | Exact payload fields             | Emission condition                        | Recipients                 |
| -------------------------- | -------------------------------- | ----------------------------------------- | -------------------------- |
| `private_message_received` | `from`, `to`, `content`, `at`    | Bilateral message validated and persisted | Sender and recipient users |
| `private_message_edited`   | `message_id`, `from`, `to`, `at` | Author edit persisted                     | Sender and recipient users |

All live connections of both participants receive these frames; co-members do
not. REST remains authoritative for message bodies after editing, attachment
metadata, history and unread positions.

### Releases

| Event                    | Exact payload fields       | Emission condition                                        | Recipients |
| ------------------------ | -------------------------- | --------------------------------------------------------- | ---------- |
| `release_step_validated` | `release_id`, `step`, `by` | Authorized next step persisted                            | Team       |
| `release_state_changed`  | `release_id`, `new_state`  | Effective state changes, including Incident block/unblock | Team       |

### Automation

| Event            | Exact payload fields                            | Emission condition                   | Recipients |
| ---------------- | ----------------------------------------------- | ------------------------------------ | ---------- |
| `rule_triggered` | `service`, `rule_name`, `result`, `incident_id` | Matching Rule completes successfully | Team       |
| `rule_failed`    | `service`, `rule_name`, `error`                 | Matching Rule's REAction fails       | Team       |

`rule_triggered.result` is `incident_created` or `reaction_completed`;
`incident_id` is `null` when no Incident was created. `rule_failed.error` is a
stable public domain code, never internal provider text.

## Change policy

One pull request must update all affected layers:

1. domain event and delivery scope in `server/src/domain/event.rs`;
2. exact serialization and shape tests in
   `server/src/adapters/ws/protocol.rs`;
3. this event table;
4. `WsServerEvent` and consumer behavior in `client-web/lib/ws.ts`;
5. routing/isolation tests when recipients change.

The step-by-step procedure is in the
[`HOWTOCONTRIBUTE.md` portal page](https://opswarden-git.github.io/opswarden/contributing/#add-a-websocket-event).
