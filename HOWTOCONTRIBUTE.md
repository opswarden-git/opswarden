# How to Contribute to OpsWarden

OpsWarden is an alpha incident-management product built as a modular monorepo:
Rust/Axum on the server, Next.js on the web, and a Tauri desktop shell. This
guide is intentionally practical: it tells you how to run the product, how the
code is organized, and what a pull request must prove before it is merged.

## Repository Map

```text
.
├── server/          Rust/Axum backend, SQLx, WebSockets
├── client-web/      Next.js 16 web client
├── client-desktop/  Tauri v2 desktop shell, URL mode in alpha
├── tooling/         tarpaulin, deny and SQLx-related config
├── .sqlx/           generated SQLx offline query cache
└── .github/         CI and release workflows
```

This repository is the implementation source of truth. A local grading audit
may exist in the sibling `../.other/` directory, but the documentation committed
here must be enough to understand and run the project on its own.

## Prerequisites

- Nix with flakes enabled.
- Docker and Docker Compose.
- GitHub CLI is useful for PR/release work, but not required for local dev.

Use the Nix shell unless you have intentionally replicated the toolchain:

```bash
nix develop
```

For the desktop shell, use the Tauri-specific shell:

```bash
nix develop .#tauri
```

## Run the Product

Start the server and database, matching the jury-friendly Docker path:

```bash
just up
```

The backend listens on `http://localhost:8080`.

Run the web client in another shell:

```bash
just web-dev
```

The web client listens on `http://localhost:4242`.

Run the desktop shell:

```bash
just desktop-dev   # wrapper for ./client-desktop/dev.sh
```

The desktop app currently runs in URL mode against `http://localhost:4242`.
Compose and CI also build and smoke-test the Linux `.deb` and AppImage packages.

## Demo Accounts

Create or restore the demo data with:

```bash
just demo
```

The command creates the demo Teams, Incidents, Releases and Automation rule. It
also creates these accounts:

| Email                        | Password | Role       |
| ---------------------------- | -------- | ---------- |
| `manager@opswarden.local`    | `sudo`   | Manager    |
| `responder@opswarden.local`  | `sudo`   | Responder  |
| `observer@opswarden.local`   | `sudo`   | Observer   |
| `contractor@opswarden.local` | `sudo`   | Non-member |

Use disposable users for verification runs, and clean them up afterwards. Do not
leave generated `*_it_*`, `e2e-*`, `verify*`, or `repro-*` accounts in the demo
database.

## Architecture Rules

The backend follows a hexagonal layout. Keep these boundaries sharp:

- `server/src/domain/` — pure business rules and domain types.
- `server/src/app/` — use-cases and orchestration over ports.
- `server/src/ports/` — traits for persistence, notifications, vault, etc.
- `server/src/adapters/` — Postgres, WebSocket hub, crypto, vault, notifier.
- `server/src/handlers/` — HTTP/WebSocket edge only; keep handlers thin.

Business decisions do not belong in React components or Axum handlers. If a rule
is testable without HTTP, it probably belongs in `domain` or `app`.

Frontend conventions:

- Use existing query hooks in `client-web/lib/queries/`.
- Keep server state in TanStack Query and invalidate precise keys after
  mutations/WebSocket events.
- Visible strings go through `client-web/messages/en.json` and
  `client-web/messages/fr.json`.
- Use existing shared UI pieces before creating a new local variant.

Desktop conventions:

- The Tauri shell is a thin wrapper over the web app in alpha.
- Native notification helpers must no-op outside Tauri.
- Tray/background behavior is desktop-only; do not leak it into web business
  logic.

## Environment

Start from the example file:

```bash
cp .env.example .env
```

Common variables:

- `DATABASE_URL` — Postgres connection string.
- `JWT_SECRET` — required in release-like runs.
- `OPSWARDEN_VAULT_KEY` — AES-GCM vault key; dev fallback exists only for local
  demos. Provider credentials and automation rules are configured inside the
  owning Team, never through global environment variables.
- `GIPHY_API_KEY` — optional server-side key for GIF search. Never expose it as
  `NEXT_PUBLIC_*`.

Never commit real secrets. `.env` is ignored.

## Extend Automation and Realtime

You do not need to build a custom frontend form for every integration. The
server publishes its Automation catalog through `/about.json`, and the web app
builds the connection and rule forms from that catalog.

Before coding, pick stable lowercase names such as `gitlab`, `pipeline_failed`
or `email_notify`. These names are stored in PostgreSQL, so renaming them later
is a data migration, not a cosmetic change.

### Add a service

A service is an integration family such as GitHub or HTTP.

1. Add its entry to `server/src/domain/automation_catalog.rs`. Declare its
   Actions, REActions and connection fields there. Use `connection: None` when
   it needs no Team credentials.
2. If it stores credentials, add the required `CredentialKind` in
   `server/src/domain/automation_config.rs`, then handle configuration in
   `server/src/app/automation/team_connection.rs` and
   `server/src/handlers/team_automation.rs`. Secrets must go through the Team
   vault; never put them in rule JSON or return them from the API.
3. Put provider-specific HTTP, payload or signature code in an adapter under
   `server/src/adapters/`. Wire a new adapter through `AppState` only when an
   existing port cannot represent it.
4. Add the English and French catalog text in `server/src/handlers/mod.rs`.
   Keep both locales structurally identical.
5. Test the catalog, credential redaction and Manager permissions. If the
   service receives webhooks, add an integration test covering a valid
   signature, an invalid signature and a duplicate delivery.

Use the `github` HMAC and `gitlab` secret-token connections as incoming-webhook
examples, and `http` as a small outgoing-connection example.

### Add an Action

An Action is an external event that can start a rule, for example
`ci_failed`.

1. Register the Action and its optional filters in
   `server/src/domain/automation_catalog.rs`.
2. Parse the provider payload in `server/src/adapters/webhook/`. Convert it to
   an `ExternalEvent` with a stable `kind` and only the non-secret attributes
   the rule engine needs.
3. Extend `IngestTeamWebhookUseCase` if the provider needs a different
   signature or credential path. An unrelated event should return `None` and
   be acknowledged as ignored, not treated as an error.
4. Add parser tests for a matching payload, an ignored payload and malformed
   input. Add an end-to-end test proving that filters select the right rule.

The frontend reads the new Action from `/about.json`. Only add React code when
the generic catalog-driven form truly cannot represent the field.

### Add a REAction

A REAction is what OpsWarden does after a rule matches, such as creating an
Incident or sending an HTTP notification.

1. Register the REAction and its fields in
   `server/src/domain/automation_catalog.rs`. Set `connection_service` when it
   needs a configured Team connection.
2. Add its execution branch to
   `server/src/app/automation/reaction_executor.rs`. Keep the orchestration in
   the app layer and network calls behind a port implemented by an adapter.
3. Read credentials from the Team vault. Rule configuration may contain normal
   values and bounded templates, but never tokens, passwords or endpoint URLs.
4. Return stable `DomainError` codes so failed runs and `rule_failed` events are
   useful to clients.
5. Test success, provider failure, invalid configuration and output limits.
   Add an integration test proving the Automation Run and WebSocket result.

Use `create_incident` for a domain-side example and `http_notify` for an
external side effect.

### Add a WebSocket event

WebSocket changes are a shared server/client contract. Update every layer in
the same pull request:

1. Add the business event to `server/src/domain/event.rs` and choose its
   delivery scope: one Team or an explicit list of users.
2. Publish it from the use-case after the database write succeeds.
3. Serialize its exact JSON shape in `server/src/adapters/ws/protocol.rs` and
   add an exact-shape Rust test.
4. Document the payload and delivery rule in `WEBSOCKET_SPEC.md`.
5. Add the event to `WsServerEvent` in `client-web/lib/ws.ts`, then handle the
   cache update, invalidation or notification it needs.
6. Add a TypeScript consumer test. If routing changed, add a server integration
   test proving that unrelated Teams or users do not receive the frame.

After any of these extensions, run the focused tests while you work, then run:

```bash
just ci
```

## Checks Before a PR

`just ci` mirrors the GitHub gate: one recipe per job, named after it. Run it
whatever the change touches.

```bash
just ci
```

It needs the stack up (`just up`) for the backend and browser jobs, and the
tooling from `nix develop` — a recipe whose tool is missing **fails** rather
than skipping, so a green run always means every check actually ran.

| Recipe                    | GitHub job                   |
| ------------------------- | ---------------------------- |
| `just ci-workflow`        | Workflow · Validate          |
| `just ci-backend-quality` | Backend · Quality & security |
| `just ci-backend-test`    | Backend · Build & test       |
| `just ci-web`             | Web · Quality & test         |
| `just ci-web-build`       | Web · Build                  |
| `just ci-e2e`             | E2E · Browser suite          |

`just ci-full` adds the two slow jobs, `Backend · Coverage` and
`Desktop (Linux) · Package`.

Useful focused checks:

```bash
just test
just lint
just fmt-check
just web-check
npm run format:check
npm run knip
```

If a backend change adds or changes SQLx queries, refresh the offline cache:

```bash
cargo sqlx prepare --workspace -- --all-targets
```

Commit the generated `.sqlx/query-*.json` changes.

For desktop changes:

```bash
nix develop .#tauri --command bash -lc 'cd client-desktop/src-tauri && cargo build'
```

For coverage:

```bash
just coverage
```

Tarpaulin executes the complete backend test suite but reports only runtime Rust
under `server/src`; `main.rs`, integration-test files and inline test functions
are excluded from the ratio. The `source-only-summary.json` gate rejects an
empty or contaminated report and enforces 70% line coverage. Vitest/V8 applies
the corresponding global runtime-source gate to the frontend.

## Database Tests

Postgres adapter tests use `#[sqlx::test]`. Each test gets an ephemeral database
that is created, migrated and dropped automatically.

That requires:

- `DATABASE_URL` to point to a Postgres server.
- The Postgres role to have the `CREATEDB` privilege.

`just test` exports a default local value:

```text
postgres://opswarden:opswarden@localhost:5433/opswarden
```

Running `cargo test` directly requires you to export `DATABASE_URL` yourself.

## Branching and Commits

Use short branches from `main`:

```text
feat/<scope>
fix/<scope>
test/<scope>
refactor/<scope>
docs/<scope>
chore/<scope>
```

Use conventional commits:

```text
feat(teams): add member role management endpoint
fix(realtime): refresh team roster on presence update
test(server): isolate PG tests and cover security adapters
```

Keep commits logical. A good PR explains:

- what changed;
- what was deliberately out of scope;
- exact commands run;
- any manual/live verification performed.

## Pull Request Definition of Done

A PR is mergeable only when:

- CI is green.
- Local checks relevant to the change were run.
- New behavior has tests or a written reason why it is display/manual only.
- Backend logic respects `domain` / `app` / `ports` / `adapters` boundaries.
- Frontend visible text is translated in both English and French.
- WebSocket/API changes update the relevant types and docs.
- SQLx query changes include the regenerated `.sqlx` cache.
- No real secrets, generated build output, or one-off scripts are committed.

## Releases

Tags `v*.*.*` trigger the release workflow. Its release gate runs Rust and web
checks first. A successful tag creates the GitHub Release, pushes the server
image to GHCR and attaches the Linux AppImage.

Before tagging:

- Update the README version badge first.
- Ensure `main` is clean and CI green.
- Write release notes that state what is proven and what is still partial.

Do not use this guide as a roadmap. Pick work from the current issue or project
board, keep the change small, and ask early when a requirement is unclear.
