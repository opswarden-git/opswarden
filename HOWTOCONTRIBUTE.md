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

Central planning docs live outside this Git repository in the sibling
`../docs/` directory. The app repository remains the implementation source of
truth; the school PDFs in `../docs/pdf/` remain the brief source of truth.

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
AppImage packaging and a self-contained desktop build are still future work.

## Demo Accounts

The clean demo database is expected to contain one team, `OpsWarden Demo`, and
three users:

| Email                      | Password       | Role      |
| -------------------------- | -------------- | --------- |
| `manager@opswarden.test`   | `DemoPass123!` | Manager   |
| `responder@opswarden.test` | `DemoPass123!` | Responder |
| `observer@opswarden.test`  | `DemoPass123!` | Observer  |

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
  demos.
- `GITHUB_WEBHOOK_SECRET` — optional bootstrap secret for GitHub webhooks.
- `OPSWARDEN_AUTOMATION_TEAM_ID` — required for automation rules to create
  incidents in a team.
- `GIPHY_API_KEY` — optional server-side key for GIF search. Never expose it as
  `NEXT_PUBLIC_*`.

Never commit real secrets. `.env` is ignored.

## Checks Before a PR

Run the full local CI mirror when the change touches backend or shared tooling:

```bash
just ci
```

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

The tarpaulin headline counts test files and therefore overstates coverage. The
latest honest backend source-only audit is still above the 70% bar; frontend test
tooling is not present yet.

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

Tags `v*.*.*` trigger the release workflow. Current release automation creates a
GitHub Release and builds/pushes the server Docker image to GHCR. Desktop
AppImage artifacts are not part of the release pipeline yet.

Before tagging:

- Update the README version badge first.
- Ensure `main` is clean and CI green.
- Write release notes that state what is proven and what is still partial.

## Current Product Gaps

Do not start portfolio/cloud/AI work before the mandatory matrix is green. The
next important product gaps are:

- Release domain and `release_blocked` notification.
- Desktop packaging/AppImage and CI artifact.
- Server-side persisted language preference, if the i18n exemption is not
  accepted.
- Jury documentation/screenshots and repo-doc reconciliation.
- Frontend automated tests.

Small documentation or demo-data cleanups are welcome when they reduce jury
noise, but they should not displace mandatory product work.
