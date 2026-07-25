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

The [repository README endpoint catalogue](https://github.com/opswarden-git/opswarden#api-and-data-model)
lists every currently exposed method and path. Route wiring in
`server/src/handlers/` is the implementation authority; update the catalogue
when that surface changes.

## Real-time follow-up

Mutations that affect collaborators may emit scoped events after persistence.
See the [WebSocket protocol](websocket.md) for envelopes, event payloads,
authorization and reconnection behavior.
