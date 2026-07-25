# Data model

The versioned migrations in `server/migrations/` are the executable source of
truth. This view explains the relations a contributor needs before changing a
use case.

```mermaid
erDiagram
    USERS ||--o{ TEAM_MEMBERS : joins
    TEAMS ||--o{ TEAM_MEMBERS : authorizes
    TEAMS ||--o{ TEAM_BANS : blocks
    TEAMS ||--o{ INCIDENTS : owns
    USERS o|--o{ INCIDENTS : assigned
    INCIDENTS ||--o{ TIMELINE_ENTRIES : records
    TIMELINE_ENTRIES ||--o{ TIMELINE_REACTIONS : receives
    INCIDENTS ||--o{ INCIDENT_EVENTS : audits
    TEAMS ||--o{ RELEASES : owns
    RELEASES ||--|{ RELEASE_STEPS : sequences
    RELEASES ||--o{ RELEASE_INCIDENTS : links
    INCIDENTS ||--o{ RELEASE_INCIDENTS : blocks
    USERS ||--o{ PRIVATE_MESSAGES : sends
    USERS ||--o{ PRIVATE_MESSAGES : receives
    TEAMS ||--o{ SERVICE_CONNECTIONS : configures
    SERVICE_CONNECTIONS ||--o{ SERVICE_CONNECTION_SECRETS : protects
    TEAMS ||--o{ AUTOMATION_RULES : owns
    SERVICE_CONNECTIONS ||--o{ WEBHOOK_DELIVERIES : authenticates
    WEBHOOK_DELIVERIES ||--o{ AUTOMATION_RUNS : triggers
    AUTOMATION_RULES o|--o{ AUTOMATION_RUNS : evaluates
```

## Invariants worth protecting

- A partial unique index permits exactly one Manager per Team.
- Kicks and bans do not rewrite historical authors or actors.
- Release steps remain ordered; linked active incidents derive the blocked state.
- Connection secrets are encrypted separately from displayable metadata.
- Webhook delivery identifiers and automation runs preserve idempotency and audit history.
- Revoked JWT identifiers persist logout invalidation.

When a migration changes one of these rules, update the domain invariant, its
integration tests and this page in the same pull request.
