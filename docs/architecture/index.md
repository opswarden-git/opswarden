# Architecture

OpsWarden is a modular hexagonal monolith. Business rules stay in one Rust
process; web and desktop clients consume the same HTTP and WebSocket contracts.
The cloud showcase remains in the separate `opswarden-ops` repository.

## Dependency rule

```mermaid
flowchart LR
    H[handlers<br/>Axum · WebSocket] --> A[app<br/>use cases]
    A --> P[ports<br/>traits]
    P --> D[domain<br/>pure rules]
    AD[adapters<br/>PostgreSQL · event bus · vault] -. implement .-> P
```

Everything points inward. The domain does not import Axum, SQLx, PostgreSQL or
network concepts.

| Layer      | Owns                                              | Must not own              |
| ---------- | ------------------------------------------------- | ------------------------- |
| `domain`   | models, transitions, invariants, stable errors    | transport or persistence  |
| `ports`    | interfaces required by use cases                  | SQL or HTTP details       |
| `app`      | authorization-aware orchestration                 | Axum responses or queries |
| `adapters` | PostgreSQL, broadcasting, encryption              | new business policy       |
| `handlers` | parsing, authentication context, response mapping | duplicated invariants     |
| clients    | interaction state and presentation                | server-side authority     |

## Request and event flow

When a user performs an action, the data flows through the system in this exact order:

1. **User action**: The Web or Desktop UI sends an authenticated HTTP command to the server.
2. **Routing & Validation**: The Axum HTTP handler receives the request, validates the basic input, and extracts the user's identity.
3. **Business Logic**: The Application use case applies the domain rules.
4. **Persistence**: The PostgreSQL adapter executes a transactional state change in the database.
5. **Event Dispatch**: Once the new state is saved, the use case publishes a domain event to the internal Event bus.
6. **Real-time Updates**: The Event bus immediately broadcasts a WebSocket event to all other connected clients so they can update their interfaces.
7. **Response**: The use case returns the result to the HTTP handler, which sends the final JSON response back to the original user.

The database change is authoritative. WebSocket events make clients converge;
they do not replace persistence or authorization.

## Repository map

```text
opswarden/
├── server/               # Rust/Axum server and all authoritative business rules
│   ├── src/
│   │   ├── domain/       # pure models and invariants; zero I/O
│   │   ├── ports/        # traits required by application use cases
│   │   ├── app/          # authorization-aware business orchestration
│   │   ├── adapters/     # PostgreSQL, event bus and encrypted vault
│   │   ├── handlers/     # Axum routes and WebSocket transport
│   │   ├── config.rs
│   │   └── lib.rs        # build_app(), testable without opening a socket
│   ├── migrations/       # executable PostgreSQL schema history
│   ├── tests/            # integration tests
│   └── Dockerfile        # multi-stage server image
├── client-web/           # Next.js interface and shared client behavior
├── client-desktop/       # Tauri URL-mode shell, tray and notifications
├── contracts/            # cross-client capabilities
├── docs/                 # source for this technical portal
├── .github/workflows/    # CI, documentation and release pipelines
├── docker-compose.yml    # db, server, desktop packager and web delivery
├── Cargo.toml            # Cargo workspace
└── package.json          # npm workspaces
```

## Why these boundaries

- **Rust, Axum and Tokio** keep lifecycle transitions typed while supporting
  concurrent HTTP and WebSocket clients.
- **PostgreSQL** supplies transactions, foreign keys and concurrent writes for
  team-scoped operational state.
- **Tauri** reuses the production web client and adds a small native shell,
  instead of creating a second product implementation.

These choices matter less than the boundary: a transport or storage change
must not require incident policy to be rewritten.
