# OpsWarden WebSocket protocol

This document is the canonical contract between the Rust server, the web
client, and the desktop client. It describes the protocol implemented on
24 August 2026.

## Conventions

- Endpoint: `GET /ws`, upgraded to WebSocket.
- Frames are UTF-8 JSON objects with a mandatory string field named `type`.
- Every identifier is a UUID encoded as a string.
- Every date is a Unix timestamp in seconds. `null` means no expiry.
- Unknown or malformed commands received after authentication are ignored.
- The server does not retain or replay WebSocket events.

## Connection lifecycle

Browser handshakes must send an `Origin` that exactly matches one entry in
`OPSWARDEN_WS_ALLOWED_ORIGINS`. A cross-site, `null`, malformed or repeated
Origin is rejected with HTTP 403 before upgrade. Originless handshakes remain
valid for native and service clients; they must still authenticate in-band.

The first text frame must be:

```json
{ "type": "auth", "token": "<access-token>" }
```

The server verifies the token and its revocation status. A missing, malformed,
invalid, or revoked token closes the connection. Ping, pong, and binary frames
may precede authentication; any other text frame fails authentication.

After authentication, the connection is scoped to the user's current teams.
One user may have several simultaneous connections. Each connection has its own
incident watches, while presence lists contain distinct user IDs.

The client reconnects automatically. On each successful reopen it must:

1. authenticate before sending another command;
2. invalidate/refetch its active REST projections;
3. replay every active `watch` command.

This REST resynchronization is mandatory because events missed while
disconnected are not replayed.

## Client commands

### `watch`

```json
{ "type": "watch", "incident_id": "<uuid>" }
```

Starts watching an incident and contributes the authenticated user to its
presence roster. The server accepts the command only when the incident exists
and belongs to one of the connection's current teams. It then broadcasts
`presence_update` to all connections watching that incident.

### `unwatch`

```json
{ "type": "unwatch", "incident_id": "<uuid>" }
```

Stops this connection from watching the incident. Removing one's own watch is
always safe and requires no resource lookup. When the roster changes, the
server broadcasts `presence_update` to the remaining watchers.

### `status_typing`

```json
{ "type": "status_typing", "incident_id": "<uuid>" }
```

Emits `user_typing` only when the incident belongs to a current team and the
user's role has the `can_signal_typing` capability. The server stores no typing
state; consumers expire the signal locally.

### `cursor`

```json
{ "type": "cursor", "incident_id": "<uuid>", "x": 0.25, "y": 0.75 }
```

Shares an ephemeral pointer position with the other connections watching the
same incident. Coordinates are finite values normalized to the incident room
(`0…1`) and are rejected outside that range. The command is accepted only after
this connection has completed an authorized `watch`; it is never persisted or
echoed to the sending connection. Clients throttle emission and expire received
positions locally.

### `refresh_teams`

```json
{ "type": "refresh_teams" }
```

Reloads the authenticated user's memberships from persistent storage and
replaces the connection's routing and authorization scope. The client sends it
after a create, join, leave, kick, ban, or delete operation that can make the
cached scope stale.

### Direct-message rooms

```json
{ "type": "watch_private_message", "peer_id": "<uuid>" }
{ "type": "unwatch_private_message", "peer_id": "<uuid>" }
{ "type": "private_message_typing", "peer_id": "<uuid>" }
```

`watch_private_message` joins the bilateral room formed by the authenticated
user and `peer_id`; `unwatch_private_message` leaves it. The pair is normalized,
so reversing the two users cannot create another room. Both users must be
distinct and currently share a Team. Watching emits `private_message_presence`
to that pair. Typing is ephemeral, authorized against the same pair and never
persisted.

## Delivery scopes

- **Team:** every live connection whose cached membership contains `team_id`.
- **Incident watchers:** every connection currently watching `resource_id`.
- **Users:** every live connection owned by one of the listed users.

Team membership and role authorization are enforced before the business action
that creates an event. Presence and typing commands are authorized again in the
WebSocket handler. Private messages are never broadcast to a team.

## Server events

The payloads below are exact. Domain-only routing fields such as `team_id` are
not added to a frame unless shown.

### Incident and timeline events

| Event                    | Exact payload fields                                      | Emission condition                                                | Delivery                |
| ------------------------ | --------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------- |
| `incident_created`       | `incident_id`, `severity`                                 | An Incident is persisted, including Automation-created Incidents. | Team                    |
| `incident_state_changed` | `incident_id`, `new_state`, `by`                          | An authorized transition changes the incident status.             | Team                    |
| `incident_escalated`     | `incident_id`, `new_severity`, `by`                       | An authorized action raises severity.                             | Team                    |
| `incident_assigned`      | `incident_id`, `assigned_to`, `by`                        | An incident is assigned.                                          | Team                    |
| `timeline_entry_added`   | `incident_id`, `entry: { entry_id, content, author, at }` | A timeline entry is persisted.                                    | Team                    |
| `timeline_entry_edited`  | `incident_id`, `entry_id`, `new_content`, `edited_at`     | An authorized edit is persisted.                                  | Team                    |
| `reaction_added`         | `incident_id`, `entry_id`, `emoji`, `by`                  | A reaction is added to an entry.                                  | Team                    |
| `reaction_removed`       | `incident_id`, `entry_id`, `emoji`, `by`                  | The caller's existing reaction is removed.                        | Team                    |
| `user_typing`            | `incident_id`, `user_id`                                  | An authorized `status_typing` command is accepted.                | Team                    |
| `cursor_update`          | `incident_id`, `user_id`, `x`, `y`                        | A watched room accepts a normalized `cursor` command.             | Other incident watchers |

Example:

```json
{
  "type": "timeline_entry_edited",
  "incident_id": "8e30dcad-f825-4670-b352-9347f8eedd11",
  "entry_id": "309667af-4484-48bc-9f9f-baa81d6868e3",
  "new_content": "Database failover completed",
  "edited_at": 1784901600
}
```

### Presence events

`presence_update` is resource-generic. Phase 1 currently emits it for incidents:

```json
{
  "type": "presence_update",
  "resource_id": "8e30dcad-f825-4670-b352-9347f8eedd11",
  "resource_type": "incident",
  "watchers": ["48173ed2-d2d9-4ef6-89ca-5ec7af5c2895", "676366a3-3325-4367-8450-1620c081b4aa"]
}
```

It is emitted after a successful watch, unwatch, or watched connection close,
and delivered only to the incident's remaining watchers.

`team_presence_update` is an OpsWarden extension:

```json
{
  "type": "team_presence_update",
  "team_id": "40b07f0c-65a6-48bd-af73-a7e81a94c275",
  "online_user_ids": ["48173ed2-d2d9-4ef6-89ca-5ec7af5c2895"]
}
```

It is emitted when a connection registers, unregisters, or refreshes its team
scope, and is delivered only to that team's live connections.

### Moderation events

```json
{
  "type": "member_kicked",
  "team_id": "40b07f0c-65a6-48bd-af73-a7e81a94c275",
  "member": "48173ed2-d2d9-4ef6-89ca-5ec7af5c2895",
  "by": "676366a3-3325-4367-8450-1620c081b4aa"
}
```

`member_kicked` is emitted after an authorized kick removes the membership and
clears the member's incident assignments.

```json
{
  "type": "member_banned",
  "team_id": "40b07f0c-65a6-48bd-af73-a7e81a94c275",
  "member": "48173ed2-d2d9-4ef6-89ca-5ec7af5c2895",
  "until": null,
  "by": "676366a3-3325-4367-8450-1620c081b4aa"
}
```

`member_banned` is emitted whenever an authorized temporary or permanent ban is
persisted, including a pre-emptive ban of a non-member. `until` contains the
expiry timestamp for a temporary ban and is `null` for a permanent ban. Both
events use Team delivery. A just-removed member remains in the live
connection's cached team scope long enough to receive the event and refresh it.

### Private messages

```json
{
  "type": "private_message_received",
  "from": "48173ed2-d2d9-4ef6-89ca-5ec7af5c2895",
  "to": "676366a3-3325-4367-8450-1620c081b4aa",
  "content": "Can you review the mitigation?",
  "at": 1784901600
}
```

The event is emitted after a message between two distinct users sharing at
least one team is validated and persisted. It is delivered through the Users
scope to exactly the sender and recipient, including all of their live
connections. Co-members and other team connections receive nothing.

The remaining direct-message frames are also delivered only to the normalized
sender/recipient pair:

```json
{
  "type": "private_message_presence",
  "participants": ["<normalized-uuid-a>", "<normalized-uuid-b>"],
  "watchers": ["<uuid>"]
}
```

```json
{ "type": "private_message_typing", "from": "<uuid>", "to": "<uuid>" }
```

```json
{
  "type": "private_message_edited",
  "message_id": "<uuid>",
  "from": "<uuid>",
  "to": "<uuid>",
  "at": 1784901600
}
```

```json
{
  "type": "private_message_reaction_changed",
  "message_id": "<uuid>",
  "from": "<uuid>",
  "to": "<uuid>",
  "emoji": "✅",
  "by": "<uuid>",
  "active": true
}
```

Presence changes when a participant watches, leaves, or disconnects. Typing
expires client-side. Edit and reaction frames are emitted only after their
authorized mutations have persisted. They are invalidation signals: REST owns
message bodies, attachment metadata and aggregate reaction state.

### Release events

| Event                    | Exact payload fields       | Emission condition                                                                        | Delivery |
| ------------------------ | -------------------------- | ----------------------------------------------------------------------------------------- | -------- |
| `release_step_validated` | `release_id`, `step`, `by` | An authorized release step is validated.                                                  | Team     |
| `release_state_changed`  | `release_id`, `new_state`  | The effective release state changes, including incident-driven block/unblock transitions. | Team     |

### Automation events

These payloads describe the currently implemented Phase 2 contract. They will
change only through a matching update to this document, Rust serialization,
TypeScript types, and tests.

| Event            | Exact payload fields                                                                  | Emission condition                                 | Delivery |
| ---------------- | ------------------------------------------------------------------------------------- | -------------------------------------------------- | -------- |
| `rule_triggered` | `service`, `rule_name`, `result`, `incident_id` (`null` when no incident was created) | A matching automation rule completes successfully. | Team     |
| `rule_failed`    | `service`, `rule_name`, `error`                                                       | A rule matches but its reaction fails.             | Team     |

`rule_triggered.result` is `incident_created` when the reaction creates an
Incident and `reaction_completed` for a successful side-effect reaction.
`rule_failed.error` is the stable public domain error code also persisted on the
Automation Run; internal error text is not sent over WebSocket.

## Contract change policy

Any protocol change must update, in the same change:

1. this document;
2. `server/src/adapters/ws/protocol.rs` and its exact-shape tests;
3. `client-web/lib/ws.ts`;
4. routing tests when a delivery scope changes;
5. consumer/cache tests when client behavior changes.
