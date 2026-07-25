# REST API

The Rust server owns authentication, authorization and domain validation. JSON
clients must treat its response and stable error codes as authoritative.

## Contract conventions

- Protected routes use `Authorization: Bearer <JWT>`.
- Authentication failures return `401`; insufficient permissions return `403`.
- An authorized request for an absent resource returns `404`.
- Domain failures use `{ "error": "…", "code": "stable_code" }`.
- Team resources require Observer, Responder or Manager capabilities depending
  on the operation; role names are not inferred client-side.
- Secrets are accepted only by connection write endpoints and never returned.

## Surface by capability

| Area                     | Route families                                           | Authority                |
| ------------------------ | -------------------------------------------------------- | ------------------------ |
| discovery                | `/health`, `/about.json`                                 | public                   |
| identity                 | `/api/auth/*`, `/api/me`                                 | public/JWT               |
| teams and moderation     | `/api/teams/*`                                           | member/Manager           |
| incidents and timeline   | `/api/incidents/*`, `/reactions/available`               | member/Responder/Manager |
| releases                 | `/api/releases/*`                                        | member/Responder/Manager |
| direct messages and GIFs | `/api/private-messages`, `/api/giphy/search`             | JWT + shared Team        |
| integrations and rules   | `/api/teams/{id}/service-connections/*`, `/automation-*` | Manager                  |
| inbound automation       | `/webhooks/github/{connection_id}`                       | signed HMAC              |

## Complete endpoint catalogue

| Method   | Route                                                                       | Access     | Purpose                                                                    |
| -------- | --------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------- |
| `GET`    | `/health`                                                                   | Public     | Liveness probe.                                                            |
| `GET`    | `/about.json?locale=en\|fr`                                                 | Public     | Server time, client host, kickoff hash and localized Automation catalogue. |
| `POST`   | `/api/auth/sign-up`                                                         | Public     | Create an email/password account.                                          |
| `POST`   | `/api/auth/sign-in`                                                         | Public     | Exchange credentials for a JWT.                                            |
| `GET`    | `/api/auth/google/start`                                                    | Public     | Start optional Google OAuth authentication.                                |
| `GET`    | `/api/auth/google/callback`                                                 | Public     | Complete Google OAuth authentication.                                      |
| `GET`    | `/api/service-oauth/github/callback`                                        | Public     | Complete a Team-scoped GitHub connection.                                  |
| `GET`    | `/api/me`                                                                   | JWT        | Read the current profile.                                                  |
| `DELETE` | `/api/me`                                                                   | JWT        | Delete the current account, subject to Manager ownership rules.            |
| `PUT`    | `/api/me/locale`                                                            | JWT        | Persist `en` or `fr` as the profile locale.                                |
| `POST`   | `/api/auth/logout`                                                          | JWT        | Revoke the current token.                                                  |
| `GET`    | `/api/giphy/search?q=…`                                                     | JWT        | Search GIFs through the server-side GIPHY proxy.                           |
| `GET`    | `/api/private-messages?peer_id=…&limit=…`                                   | JWT        | Read one bilateral conversation.                                           |
| `POST`   | `/api/private-messages`                                                     | JWT        | Send at most 2,000 characters to a user sharing a Team.                    |
| `GET`    | `/api/teams`                                                                | JWT        | List the current user's Teams and operational counts.                      |
| `POST`   | `/api/teams`                                                                | JWT        | Create a Team; its creator becomes the sole Manager.                       |
| `POST`   | `/api/teams/join`                                                           | JWT        | Join a Team using a valid invitation code.                                 |
| `DELETE` | `/api/teams/{team_id}`                                                      | Manager    | Delete a Team and its owned resources.                                     |
| `POST`   | `/api/teams/{team_id}/leave`                                                | JWT        | Leave a Team; its Manager must transfer ownership first.                   |
| `PUT`    | `/api/teams/{team_id}/manager`                                              | Manager    | Atomically transfer the single Manager role.                               |
| `GET`    | `/api/teams/{team_id}/invitation`                                           | Manager    | Read the invitation code.                                                  |
| `GET`    | `/api/teams/{team_id}/members`                                              | Member     | List members, roles and presence data.                                     |
| `PUT`    | `/api/teams/{team_id}/members/{user_id}/role`                               | Manager    | Set an Observer or Responder role.                                         |
| `DELETE` | `/api/teams/{team_id}/members/{user_id}`                                    | Manager    | Kick a member without rewriting history.                                   |
| `GET`    | `/api/teams/{team_id}/bans`                                                 | Manager    | List temporary and permanent bans.                                         |
| `POST`   | `/api/teams/{team_id}/bans`                                                 | Manager    | Ban a member temporarily or permanently.                                   |
| `DELETE` | `/api/teams/{team_id}/bans/{user_id}`                                       | Manager    | Lift a ban explicitly.                                                     |
| `GET`    | `/api/incidents`                                                            | JWT        | List accessible Incidents, optionally filtered by Team.                    |
| `POST`   | `/api/incidents`                                                            | Manager    | Create an Incident.                                                        |
| `GET`    | `/api/incidents/{incident_id}`                                              | Member     | Read an Incident.                                                          |
| `DELETE` | `/api/incidents/{incident_id}`                                              | Manager    | Permanently delete an Incident after confirmation.                         |
| `PUT`    | `/api/incidents/{incident_id}/status`                                       | Responder+ | Apply a valid lifecycle transition.                                        |
| `PUT`    | `/api/incidents/{incident_id}/assign`                                       | Manager    | Assign a Responder or Manager.                                             |
| `GET`    | `/api/incidents/{incident_id}/activity`                                     | Member     | Read the unified Incident activity stream.                                 |
| `POST`   | `/api/incidents/{incident_id}/timeline`                                     | Responder+ | Add a timeline entry.                                                      |
| `PUT`    | `/api/incidents/{incident_id}/timeline/{entry_id}`                          | Author     | Edit an owned entry while preserving its original timestamp.               |
| `POST`   | `/api/incidents/{incident_id}/timeline/{entry_id}/reactions`                | Member     | Toggle a supported emoji reaction.                                         |
| `GET`    | `/reactions/available`                                                      | JWT        | Read the server-owned reaction catalogue: 👍, 👀, ✅, 🚨, ❤️, 🎉.          |
| `GET`    | `/api/releases`                                                             | JWT        | List accessible Releases, optionally filtered by Team.                     |
| `POST`   | `/api/releases`                                                             | Manager    | Create a Release with ordered steps.                                       |
| `GET`    | `/api/releases/{id}`                                                        | Member     | Read a Release, its steps and linked Incidents.                            |
| `POST`   | `/api/releases/{id}/cancel`                                                 | Manager    | Cancel a Release.                                                          |
| `POST`   | `/api/releases/{id}/steps/{step}/validate`                                  | Responder+ | Validate the next available step.                                          |
| `POST`   | `/api/releases/{id}/incidents/{incident_id}/link`                           | Manager    | Link an Incident and derive blocking state.                                |
| `DELETE` | `/api/releases/{id}/incidents/{incident_id}/link`                           | Manager    | Unlink an Incident.                                                        |
| `GET`    | `/api/teams/{team_id}/service-connections`                                  | Manager    | List connection metadata without secret material.                          |
| `PUT`    | `/api/teams/{team_id}/service-connections/by-service/{service}`             | Manager    | Configure a catalogue-driven service connection.                           |
| `PUT`    | `/api/teams/{team_id}/service-connections/github`                           | Manager    | Configure GitHub credentials or webhook signing.                           |
| `PUT`    | `/api/teams/{team_id}/service-connections/http`                             | Manager    | Configure a bounded HTTP notification destination.                         |
| `POST`   | `/api/teams/{team_id}/service-connections/by-service/{service}/oauth/start` | Manager    | Start service OAuth with state and PKCE.                                   |
| `POST`   | `/api/teams/{team_id}/service-connections/{connection_id}/oauth/refresh`    | Manager    | Rotate encrypted GitHub OAuth credentials.                                 |
| `POST`   | `/api/teams/{team_id}/service-connections/{connection_id}/test`             | Manager    | Test an HTTP connection without exposing its endpoint.                     |
| `DELETE` | `/api/teams/{team_id}/service-connections/{connection_id}`                  | Manager    | Delete a connection and its encrypted credentials.                         |
| `GET`    | `/api/teams/{team_id}/automation-rules`                                     | Manager    | List Action→REAction rules.                                                |
| `POST`   | `/api/teams/{team_id}/automation-rules`                                     | Manager    | Create a disabled-by-default rule.                                         |
| `PATCH`  | `/api/teams/{team_id}/automation-rules/{rule_id}`                           | Manager    | Update or enable a rule.                                                   |
| `DELETE` | `/api/teams/{team_id}/automation-rules/{rule_id}`                           | Manager    | Delete a rule while preserving run history.                                |
| `GET`    | `/api/teams/{team_id}/automation-runs`                                      | Manager    | List Automation executions and outcomes.                                   |
| `POST`   | `/webhooks/github/{connection_id}`                                          | HMAC       | Receive a size-limited, signed GitHub event idempotently.                  |

Route wiring in `server/src/handlers/` is the implementation authority. Update
this catalogue in the same pull request whenever that surface changes.

## Real-time follow-up

Mutations that affect collaborators may emit scoped events after persistence.
See the [WebSocket protocol](websocket.md) for envelopes, event payloads,
authorization and reconnection behavior.
