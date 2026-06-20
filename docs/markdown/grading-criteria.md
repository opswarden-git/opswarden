# Grading criteria — cumulative RTC 1 + RTC 2 + VIGIL

This file is an audit matrix, not a promise. The source of truth is:

- `docs/pdf/01_consignes_RTC.pdf` — original RTC project.
- `docs/pdf/02_consignes_RTC.pdf` — RTC Strikes Back extension.
- `docs/pdf/03_consignes_VIGIL.pdf` — VIGIL rattrapage.
- The official grading rubrics copied from the school platform for RTC 1 and RTC 2.

See `docs/markdown/official-sources.md` for PDF hashes and update policy.

Interpretation for VIGIL: RTC 1 and RTC 2 are past projects, but VIGIL asks the
remaining T-DEV-600 achievements to be covered again under the VIGIL product
constraints. OpsWarden therefore maps the chat vocabulary to the operational
control-room vocabulary:

| RTC term               | VIGIL/OpsWarden equivalent           |
| ---------------------- | ------------------------------------ |
| server / community     | Team                                 |
| channel                | Incident war room / operational room |
| message                | Timeline entry                       |
| online / typing status | Presence / typing on an incident     |
| owner / admin / member | Manager / Responder / Observer       |

Status legend:

- `OK` — implemented in code and locally checked.
- `PARTIAL` — partially implemented, backend-only, missing UI, missing proof, or not final.
- `KO` — not implemented in the current codebase.
- `N/A yet` — intentionally deferred until prerequisite scope exists.

Audit date: 2026-06-19.

## Verification Snapshot

Local checks run on 2026-06-19:

| Check                                                                                 | Result                |
| ------------------------------------------------------------------------------------- | --------------------- |
| `cargo test --workspace`                                                              | OK — 153 tests passed |
| `cargo fmt --all -- --check`                                                          | OK                    |
| `nix develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings` | OK                    |
| `npm run typecheck --workspace client-web`                                            | OK                    |
| `npm run lint --workspace client-web`                                                 | OK                    |
| `npm run format:check --workspace client-web`                                         | OK                    |
| `npm run build --workspace client-web`                                                | OK                    |

Known caveat: coverage percentage was not measured in this audit.

2026-06-20 update: realtime hardening (WS cross-team authz on `watch`/`status_typing`,
per-incident presence/typing store, timeline polling removed) was merged and the live
2-browser + cross-team scenario was proven; `v0.1.0` is tagged on `main` (`9d18c55`).
Re-verified this session: `tsc`, ESLint, Prettier, `cargo test -p opswarden-server`,
`clippy -D warnings`, `cargo fmt --check`. The full `--workspace` test/build suite from
the table above was not re-run.

## RTC 1 Rubric

| ID                    | Criterion                                                          | Status  | Evidence / gap                                                                                                        |
| --------------------- | ------------------------------------------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------- |
| `specs_server`        | Server uses NodeJS or Rust and allows simultaneous connections     | OK      | Rust/Axum server, `/ws`, async handlers                                                                               |
| `specs_client`        | Client uses ReactJS or NextJS and is connected to the server       | OK      | Next.js client builds and calls `/api/*`                                                                              |
| `user_list`           | Users can see who joined the server                                | KO      | No team member list endpoint/UI yet                                                                                   |
| `chan_list`           | Users can list all channels inside a server                        | OK      | mapped to incident list per team                                                                                      |
| `server_create`       | Users can create a server                                          | OK      | mapped to team creation, API + UI                                                                                     |
| `server_delete`       | Users can delete a server                                          | PARTIAL | backend team delete exists, no final UI flow/confirmation                                                             |
| `server_join`         | Users can join a server                                            | OK      | invitation-code join, API + UI                                                                                        |
| `server_multiple`     | Users can join multiple servers simultaneously                     | OK      | membership model and team list support multiple teams                                                                 |
| `server_quit`         | Users can leave a server                                           | PARTIAL | backend leave exists, no final UI flow                                                                                |
| `chan_create`         | Users can create a channel inside a server                         | OK      | mapped to incident creation, API + UI                                                                                 |
| `chan_delete`         | Users can delete a channel                                         | PARTIAL | backend incident delete exists, no final UI flow/confirmation                                                         |
| `chan_message`        | Users can send a message to all users in a channel using WebSocket | OK      | Timeline WS events broadcast to and update all co-watchers in real-time                                               |
| `status_online`       | Users can see who is online on the server                          | PARTIAL | incident watcher presence exists; no full team-online roster                                                          |
| `status_typing`       | Users can see who is typing inside a channel                       | OK      | typing broadcast to the incident's team, guarded by a membership check; strict co-watcher scoping not yet implemented |
| `user_management`     | Different roles allow different permissions inside a server        | PARTIAL | RBAC + Manager transfer exist; general member role management UI absent                                               |
| `persistency`         | Servers, channels and messages are persisted                       | OK      | Postgres users/teams/incidents/timeline/vault                                                                         |
| `functional-delivery` | Delivery is functional, most previous achievements obtained        | PARTIAL | backend and web build pass; cumulative feature scope incomplete                                                       |
| `ui_servers`          | Server management interface is clear and intuitive                 | PARTIAL | Teams screen exists; delete/leave/member management missing                                                           |
| `ui_chat`             | Chat interface inside a channel is clear and intuitive             | PARTIAL | Timeline UI exists; accessibility gap on placeholder-only input                                                       |
| `ui_design`           | Interface design is elaborated and advanced                        | PARTIAL | visual system exists; screenshots/final polish missing                                                                |
| `uiux_quality`        | Delivery offers high-quality UX/UI                                 | PARTIAL | usable foundation, known accessibility and parity gaps                                                                |
| `versioning_basics`   | Proper versioning workflow, branching, commits, `.gitignore`       | OK      | repo initialized, `.gitignore`, PR template/workflows present                                                         |
| `coding_style`        | Code respects a common coding style                                | OK      | fmt, clippy, ESLint, Prettier pass                                                                                    |
| `tests_unit`          | At least 70% of source code tested                                 | PARTIAL | many tests pass; 70% coverage not measured yet                                                                        |
| `tests_automation`    | Tests are easily runnable                                          | OK      | `cargo test --workspace`, npm checks, CI workflow                                                                     |
| `tests_coverage`      | Branches tested beyond main flow                                   | PARTIAL | many error-path tests exist; coverage report not audited                                                              |
| `documentation`       | README and newcomer docs delivered                                 | PARTIAL | README exists but has stale/promissory sections                                                                       |
| `presentation`        | Professional presentation support/demo                             | KO      | not present in repo                                                                                                   |
| `extra_small`         | At least 1 feature outside RTC features                            | PARTIAL | automation/OAuth exist, but do not count before core is stable                                                        |
| `extra_medium`        | At least 3 extra features outside RTC features                     | PARTIAL | same caveat                                                                                                           |
| `extra_large`         | More than 4 extra features outside RTC features                    | KO      | defer until mandatory criteria are green                                                                              |

## RTC 2 Rubric

| ID                      | Criterion                                                      | Status  | Evidence / gap                                                               |
| ----------------------- | -------------------------------------------------------------- | ------- | ---------------------------------------------------------------------------- |
| `milestone_1`           | First milestone complete                                       | KO      | kick/temp ban/permanent ban/message editing not implemented                  |
| `milestone_2`           | Second milestone complete                                      | KO      | PM, reactions, GIF/external API criterion incomplete                         |
| `milestone_3`           | Third milestone complete                                       | KO      | desktop app absent                                                           |
| `web_server`            | Server uses NodeJS or Rust and allows simultaneous connections | OK      | Rust/Axum + WebSocket                                                        |
| `web_client`            | Client uses ReactJS or NextJS and is connected                 | OK      | Next.js app builds and calls API                                             |
| `web_core_features`     | Kick, temp/permanent bans, message editing                     | KO      | not implemented                                                              |
| `web_multilingual`      | Web app can switch between at least two languages              | OK      | `next-intl`, `en`/`fr`, settings switch                                      |
| `web_api_integration`   | External GIF API integrated                                    | KO      | no GIF API; VIGIL automation does not automatically satisfy this rubric      |
| `web_pm`                | Private messages between users                                 | KO      | not implemented                                                              |
| `web_reactions`         | Emoji reactions on messages                                    | KO      | not implemented                                                              |
| `desktop_app`           | Runnable functional desktop app                                | KO      | no `client-desktop/`                                                         |
| `desktop_specs`         | Desktop uses Tauri or Electron and connects to server          | KO      | no desktop app                                                               |
| `desktop_multilingual`  | Desktop translated into at least two languages                 | KO      | no desktop app                                                               |
| `desktop_notifications` | Desktop notification system                                    | KO      | no desktop app                                                               |
| `tests_unit`            | At least 70% source code tested                                | PARTIAL | tests pass; coverage not measured                                            |
| `tests_sequence`        | Runnable test sequence delivered                               | OK      | `Justfile`, cargo/npm checks, CI                                             |
| `tests_automation`      | Test sequence launched by CI pipeline                          | OK      | `.github/workflows/ci.yml`                                                   |
| `tests_coverage`        | Tested-code proportion delivered                               | PARTIAL | CI has tarpaulin artifact on `main`; current percentage unknown              |
| `repo_versioning`       | Version control workflow                                       | OK      | `.gitignore`, PR template, workflows                                         |
| `repo_secrets`          | Secrets not committed in clear text                            | PARTIAL | `.env` ignored; dev defaults and local `server/.env` must be watched         |
| `repo_cicd`             | Tests and build on tag                                         | PARTIAL | CI and release workflow exist; tag release does not build desktop binary yet |
| `repo_doc`              | README and newcomer docs                                       | PARTIAL | present but stale in places                                                  |
| `code_style`            | Best practices and consistent standards                        | OK      | local checks pass                                                            |
| `code_maintainability`  | Maintainable code structure                                    | OK      | server is hexagonal with ports/adapters/use-cases                            |
| `proj_pres`             | Professional presentation                                      | KO      | not present                                                                  |
| `proj_review`           | One feature reviewed in presentation                           | KO      | not prepared                                                                 |
| `proj_answers`          | Can answer jury questions                                      | N/A yet | requires prep                                                                |
| `proj_orga`             | Proof of organization                                          | PARTIAL | commits/workflows/docs exist; board not audited                              |
| `extra_small`           | At least 1 extra feature                                       | PARTIAL | do not prioritize before mandatory scope                                     |
| `extra_medium`          | At least 3 extra features                                      | PARTIAL | do not prioritize before mandatory scope                                     |
| `extra_large`           | More than 5 extra features                                     | KO      | deferred                                                                     |

## VIGIL Phase Criteria

### Phase 1 — Core

| Criterion                                                                | Status  | Evidence / gap                                                                                                    |
| ------------------------------------------------------------------------ | ------- | ----------------------------------------------------------------------------------------------------------------- |
| Email/password auth, `GET /me`, sign out with token invalidation         | OK      | API + tests                                                                                                       |
| Teams with Observer/Responder/Manager, invitation code, Manager transfer | OK      | API/domain/tests                                                                                                  |
| Incidents lifecycle, severities, real-time collaborative timeline        | OK      | Lifecycle, severities, and timeline are fully functional with two-browser updates                                 |
| Persistence of all data                                                  | PARTIAL | implemented data is persisted; releases/PM/reactions/moderation absent                                            |
| WebSockets core events                                                   | OK      | server protocol + client hook for incident/timeline/presence                                                      |
| Automatic client-side reconnection                                       | OK      | `react-use-websocket` reconnect + re-auth; full state resync-on-reopen tracked in S2.5 (timeline polling removed) |

### Phase 1 — Extended / Final Jury Value

| Criterion                                  | Status  | Evidence / gap                               |
| ------------------------------------------ | ------- | -------------------------------------------- |
| OAuth2 sign-in                             | PARTIAL | Google OAuth exists; VIGIL recommends GitHub |
| Releases with blocking by linked incidents | KO      | not implemented                              |
| Member moderation                          | KO      | not implemented                              |
| Timeline entry editing                     | KO      | not implemented                              |
| Private messages                           | KO      | not implemented                              |
| Timeline reactions                         | KO      | not implemented                              |
| Extended WebSocket events                  | KO      | not implemented                              |

### Phase 2 — Core

| Criterion                                                                     | Status  | Evidence / gap                                                                                  |
| ----------------------------------------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------- |
| Rule engine: 1 external Action, VIGIL + 1 additional REAction, 1 working rule | PARTIAL | GitHub Action + create incident + HTTP notify exist; `/about.json` exposes only create_incident |
| `/about.json` dynamic service catalog + SHA-256 token                         | PARTIAL | token OK; catalog is static and incomplete                                                      |
| WebSockets `rule_triggered`, `rule_failed`                                    | OK      | server protocol + tests                                                                         |
| Server-side encrypted storage of tokens                                       | PARTIAL | AES-GCM vault exists; no user-facing service connection flow                                    |
| CI/CD pipeline functional                                                     | PARTIAL | CI exists and checks pass locally; release lacks desktop artifact                               |

### Phase 2 — Extended

| Criterion                                | Status  | Evidence / gap                                                                    |
| ---------------------------------------- | ------- | --------------------------------------------------------------------------------- |
| Web i18n FR/EN                           | OK      | implemented with `next-intl`                                                      |
| Linter, formatter, coverage report in CI | PARTIAL | lint/format CI exist; coverage only on main and not audited                       |
| Additional services beyond minimum       | KO      | not implemented beyond GitHub + generic notify                                    |
| Detailed `HOWTOCONTRIBUTE.md`            | KO      | current file is generic and does not explain service/Action/REAction/WS extension |

### Phase 3 — Core

| Criterion                                                            | Status  | Evidence / gap                                        |
| -------------------------------------------------------------------- | ------- | ----------------------------------------------------- |
| Installable desktop app                                              | KO      | no desktop app                                        |
| Native notifications: assignment, critical severity, blocked release | KO      | no desktop app; releases absent                       |
| Docker Compose exposes desktop binary via web client                 | KO      | compose currently has only `db` and `server`          |
| Complete README                                                      | PARTIAL | exists but not complete and contains stale statements |

### Phase 3 — Extended

| Criterion                                       | Status  | Evidence / gap                                              |
| ----------------------------------------------- | ------- | ----------------------------------------------------------- |
| `WEBSOCKET_SPEC.md` complete and current        | PARTIAL | now code-backed for implemented events; final events absent |
| `HOWTOCONTRIBUTE.md` complete and current       | KO      | must be rewritten                                           |
| `UI_GUIDELINES.md` with 2 annotated screenshots | PARTIAL | code-backed but screenshots absent                          |
| Desktop binary build verified in CI             | KO      | no desktop app/build                                        |

## Priority Rule

Do not spend project time on cloud deployment, marketing website, AI SRE/RAG, or
portfolio-only features until the cumulative RTC 1 + RTC 2 + VIGIL mandatory
criteria above are green or explicitly waived by written evidence.
