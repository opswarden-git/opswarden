# Contributing

This guide defines how to change OpsWarden safely. Installation, environment
variables and normal startup commands belong in the
[`README.md` portal page](https://opswarden-git.github.io/opswarden/); visual
policy belongs in
[`UI_GUIDELINES.md`](https://opswarden-git.github.io/opswarden/design/ui-guidelines/),
token details in
[`DESIGN_SYSTEM.md`](https://opswarden-git.github.io/opswarden/design/design-system/),
and the realtime wire contract in
[`WEBSOCKET_SPEC.md`](https://opswarden-git.github.io/opswarden/reference/websocket/).

## Start a change

Use the reproducible development shell:

```bash
nix develop
```

Use `nix develop .#tauri` for desktop work. Create a short branch from `main`
using `feat/<scope>`, `fix/<scope>`, `test/<scope>`, `refactor/<scope>`,
`docs/<scope>` or `chore/<scope>`. Keep commits logical and use conventional
commit messages, for example `fix(realtime): refresh presence after joining`.

Before coding, locate the owning layer:

| Area                        | Responsibility                                  |
| --------------------------- | ----------------------------------------------- |
| `server/src/domain/`        | Pure business rules and domain types            |
| `server/src/app/`           | Use-cases and orchestration through ports       |
| `server/src/ports/`         | Interfaces for persistence and external effects |
| `server/src/adapters/`      | PostgreSQL, WebSocket, crypto and providers     |
| `server/src/handlers/`      | Thin HTTP/WebSocket transport edge              |
| `client-web/lib/queries/`   | Server state, mutations and cache keys          |
| `client-web/components/ui/` | Shared accessible UI primitives                 |
| `client-desktop/`           | Thin Tauri shell and native capabilities        |

Business decisions belong on the server, not in React components or handlers.
Visible copy must exist in both `client-web/messages/en.json` and `fr.json`.
Use an existing query hook and shared component before creating a local wrapper.
Native helpers must safely no-op outside Tauri.

Never commit credentials. Provider tokens are Team-owned secrets encrypted by
the server; they do not belong in rule JSON, client code, logs or API responses.

## Extend Automation

The server owns the Automation catalog exposed through `/about.json`; clients
build forms from it. Choose stable lowercase service/event names because they
are persisted and renaming them requires a migration.

### Add a Service

A Service is an integration family such as GitHub, GitLab, HTTP or Email.

1. Register the Service, its Actions, REActions and connection fields under
   `server/src/domain/automation_catalog/`.
2. If credentials are needed, add their `CredentialKind` in
   `server/src/domain/automation_config.rs`; configure them through
   `server/src/app/automation/team_connection.rs` and the Team Automation
   handlers. Store and read values only through the encrypted Team vault.
3. Put provider-specific transport, signature and payload behavior in
   `server/src/adapters/`; add a port when no existing interface represents the
   external effect.
4. Add structurally identical English and French catalog descriptions in the
   About handler.
5. Test catalog exposure, Manager authorization, credential redaction and Team
   isolation. Incoming webhooks also require valid-authentication,
   invalid-authentication and duplicate-delivery cases.

Use GitHub/GitLab as signed incoming examples and HTTP as an outgoing example.
Do not add a custom frontend form unless the catalog schema cannot express the
field.

### Add an Action

An Action is an external event that can trigger a Rule, such as `ci_failed`.

1. Register the Action and its bounded filters in the Service catalog.
2. Parse the provider payload under `server/src/adapters/webhook/` and normalize
   it to an `ExternalEvent` containing a stable `kind` and non-secret attributes.
3. Reuse `IngestTeamWebhookUseCase`; extend its authentication path only when
   the provider contract requires it. A valid but unrelated event returns
   `None` and is recorded as ignored.
4. Test matching, ignored and malformed payloads, authentication,
   retry/idempotency and an end-to-end Rule selected by its filters.

### Add a REAction

A REAction is the server-side effect executed after a Rule matches.

1. Register the REAction and its fields in the catalog. Declare
   `connection_service` when it depends on a Team connection.
2. Add orchestration to
   `server/src/app/automation/reaction_executor.rs`; network calls remain behind
   a port and adapter.
3. Read credentials from the Team vault. Rule configuration may contain bounded
   values and templates, never tokens, passwords or secret endpoint URLs.
4. Return stable public `DomainError` codes for Automation Runs and
   `rule_failed`; do not expose internal provider errors.
5. Test success, provider failure, invalid configuration, output limits,
   durable Run state and the resulting WebSocket event.

`create_incident` is the domain-side reference; `http_notify` demonstrates an
external side effect.

## Add a WebSocket event

A WebSocket change is one atomic server/client/documentation contract:

1. Add the business event to `server/src/domain/event.rs` and select Team or
   explicit-user delivery.
2. Publish it from the use-case only after the database mutation succeeds.
3. Serialize the exact frame in `server/src/adapters/ws/protocol.rs` and add an
   exact-shape Rust test.
4. Add the event, exact payload fields, emission condition and recipients to
   `WEBSOCKET_SPEC.md`.
5. Add the TypeScript type to `WsServerEvent` in `client-web/lib/ws.ts` and
   implement its cache/store/notification effect.
6. Add a TypeScript consumer test. When delivery changes, add a server test
   proving unrelated Teams or users receive nothing.

Do not add private-message reactions: VIGIL limits reactions to Incident
timeline entries.

## Validate the change

Run focused tests while iterating, then reproduce the GitHub gate:

```bash
just ci
```

The stack must be running with `just up`. A missing tool fails rather than
silently skipping a check. `just ci-full` additionally runs backend coverage
and the Linux desktop package job.

Useful focused commands:

```bash
just test
just lint
just fmt-check
just web-check
npm run format:check
npm run knip
```

For SQLx query changes, run:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

Commit the updated `.sqlx/query-*.json` cache. PostgreSQL adapter tests use
`#[sqlx::test]` and require a `DATABASE_URL` whose role can create temporary
databases; `just test` supplies the documented local default.

Coverage is measured by `just coverage`: Tarpaulin enforces 70% line coverage
for runtime Rust under `server/src`, while Vitest/V8 enforces the frontend
runtime-source thresholds. Do not weaken scopes or thresholds to make a change
pass.

## Pull request contract

A pull request is ready only when:

- relevant local checks and the required CI gate are green;
- behavior has tests, including negative authorization and isolation cases;
- server boundaries remain intact and visible strings are translated EN/FR;
- REST, WebSocket, schema and documentation changes are updated together;
- changed SQLx queries include their offline cache;
- no secret, build output or one-off verification script is committed;
- the description records scope, excluded work, commands run and manual checks.

Repository guardrails enforce source files below 500 lines, forbid TypeScript
escape hatches, pin Docker base images, preserve released migrations and reject
unrelated test deletion. Executable policy lives in
`tooling/check_source_hygiene.sh`, `tooling/check_dockerfile_pins.sh` and
`tooling/check_migration_policy.sh`; these scripts, not duplicated prose, are
authoritative.

Keep this guide procedural. Product behavior belongs in the README and protocol
documents; planned work belongs in the issue tracker or roadmap.
