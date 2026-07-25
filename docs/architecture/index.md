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

```mermaid
sequenceDiagram
    participant UI as Web / desktop UI
    participant HTTP as Axum handler
    participant UC as Application use case
    participant DB as PostgreSQL adapter
    participant BUS as Event bus adapter
    participant WS as Connected clients

    UI->>HTTP: authenticated command
    HTTP->>UC: validated input + actor
    UC->>DB: transactional state change
    DB-->>UC: persisted aggregate
    UC->>BUS: domain event
    BUS-->>WS: scoped WebSocket event
    UC-->>HTTP: result
    HTTP-->>UI: JSON response
```

The database change is authoritative. WebSocket events make clients converge;
they do not replace persistence or authorization.

## Repository map

```text
server/src/domain/     pure models and invariants
server/src/ports/      traits required by application code
server/src/app/        use cases
server/src/adapters/   PostgreSQL, event bus and vault implementations
server/src/handlers/   Axum routes and WebSocket transport
client-web/            Next.js interface and shared UI behavior
client-desktop/        Tauri URL-mode shell and native notifications
contracts/             cross-client capabilities
server/migrations/     executable PostgreSQL schema history
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
