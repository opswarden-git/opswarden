<p align="center">
  <img src="client-web/public/assets/heroicon.png" alt="OpsWarden" width="130" />
  <h1 align="center">OpsWarden</h1>
</p>

<p align="center">
  <a href="https://github.com/RomeoCavazza/opswarden/actions/workflows/ci.yml"><img src="https://github.com/RomeoCavazza/opswarden/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="https://github.com/RomeoCavazza/opswarden/releases/latest"><img src="https://img.shields.io/badge/release-v0.2.0-F4C430?style=flat-square" alt="Release v0.2.0" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-F4C430?style=flat-square" alt="License: Apache 2.0" /></a>
  <img src="https://img.shields.io/badge/status-alpha-2F2F2F?style=flat-square" alt="Status: alpha" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Axum-2F2F2F?style=flat-square" alt="Axum" />
  <img src="https://img.shields.io/badge/Tokio-2F2F2F?style=flat-square" alt="Tokio" />
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL" />
  <img src="https://img.shields.io/badge/Next.js-000000?style=flat-square&logo=nextdotjs&logoColor=white" alt="Next.js" />
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Tailwind_CSS-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white" alt="Tailwind CSS" />
  <img src="https://img.shields.io/badge/Tauri-24C8DB?style=flat-square&logo=tauri&logoColor=white" alt="Tauri" />
  <img src="https://img.shields.io/badge/Docker-2496ED?style=flat-square&logo=docker&logoColor=white" alt="Docker" />
  <img src="https://img.shields.io/badge/GitHub_Actions-2088FF?style=flat-square&logo=githubactions&logoColor=white" alt="GitHub Actions" />
</p>

---

- [Scope](#scope) — what ships, in tiers
- [How it works](#how-it-works) — install and run locally
- [Architecture](#architecture) — hexagonal, where things live
- [Roadmap](#roadmap) — project milestones
- [Contributing](#contributing) — workflow and Definition of Done

## Introduction

**OpsWarden** is a platform where a technical team coordinates, in real time, its
**Incidents** (unplanned problems, triaged and resolved) and its **Releases**
(deployments validated step by step). The two are linked: an active incident can
block an in-progress release.

External events (a failing GitHub CI run, a GitLab webhook) automatically trigger
internal actions through an **Action&rarr;REAction** rule engine, and an **AI SRE**
investigation agent reads the context (logs, commit diff, similar past incidents
via vector search) to propose a root-cause hypothesis and a runbook, posted
straight into the incident timeline.

Positioning: a publishable mini incident.io / Rootly focused on reducing MTTR,
rather than yet another re-skinned real-time chat. All business logic lives on the
server (Rust/Axum, hexagonal architecture); the web and desktop clients display
and relay, with no business logic.

> Status: the **core backend** is implemented and tested — email/JWT auth (with
> logout/revocation), teams + 3-role RBAC, the incident lifecycle and real-time
> timeline, all on PostgreSQL (SQLx). The real-time front-end and later phases are
> in progress; this page describes the full target and how to get there.

## Scope

OpsWarden aims to be a real Incident Management Platform, in the lineage of
**PagerDuty, Opsgenie, incident.io, Rootly and Datadog Incident Management**,
delivered in tiers. Locked architecture decision: a modular hexagonal monolith
(cargo + npm workspaces) for the core, **a single extracted service** (the AI SRE
agent, behind a port), and the cloud/ops layer in **separate repositories** — the
microservices instinct is honored where it pays, without distributed-systems tax.

**Core Features**

- Email auth + JWT, `/me`, logout with token invalidation; teams + 3-role RBAC
  (Observer / Responder / Manager) + invitation code + Manager transfer
- Incidents (open &rarr; acknowledged &rarr; escalated &rarr; resolved, severities)
  with a real-time collaborative timeline
- WebSockets (`incident_*`, `presence_update`) + automatic client reconnection
- Action&rarr;REAction automation: GitHub webhook (CI failed) &rarr; incident;
  dynamic `/about.json` + SHA-256 token; encrypted token vault (AES-GCM)
- Tauri desktop client (OS notifications, tray); `docker-compose` (server 8080 /
  client_web 8081 / db / exposed desktop binary); GitHub Actions CI/CD; FR/EN i18n

**Extended Features**

- GitHub OAuth2; Releases + automatic blocking by a linked incident; moderation
  (kick / temp ban / perm ban); timeline editing, reactions, private messages
- GitLab as an Action; additional REActions (Slack / HTTP / Email)

**Long-term vision**

- **AI SRE**: RAG microservice (FastAPI, `@ask` / `@search`, pgvector, LLM/SLM)
  correlating logs + commit diff + past incidents to propose a root cause + runbook
- **Integrations**: Slack, Jira / Confluence
- **Observability**: OpenTelemetry + Prometheus + Grafana + Loki + Promtail
- **IaC showcase** (repo `opswarden-ops`): Minikube &rarr; k8s &rarr; Terraform &rarr;
  DigitalOcean (DOKS) + Traefik + cAdvisor + Argo/Flux; Redis + async workers
- **Deployment**: Vercel (web) + multi-repo (product monorepo v1, separate ops repos)

## How it works

### Installation

```bash
# 1. Clone
git clone https://github.com/RomeoCavazza/opswarden.git    # HTTPS
git clone git@github.com:RomeoCavazza/opswarden.git         # SSH
cd opswarden

# 2. Configure the environment
cp .env.example .env
# adjust OPSWARDEN_KICKOFF_TOKEN and DATABASE_URL if needed

# 3. Run everything (server + database)
docker compose up --build
```

Check the server responds:

```bash
curl http://localhost:8080/health      # -> {"status":"ok"}
curl http://localhost:8080/about.json  # -> service catalog + SHA-256 token
```

### The project at a glance

```text
opswarden/
├── server/               # Rust/Axum -- ALL business logic (hexagonal)
│   ├── src/
│   │   ├── domain/       # pure models (Incident, Release, Team...) -- zero I/O
│   │   ├── ports/        # traits (IncidentRepo, EventBus, TokenVault...)
│   │   ├── app/          # use-cases (business rule orchestration)
│   │   ├── adapters/     # port implementations (Postgres, WS, crypto)
│   │   ├── handlers/     # Axum routes + WebSocket upgrade (no logic)
│   │   ├── config.rs
│   │   └── lib.rs        # build_app(): app testable without opening a socket
│   ├── tests/            # integration tests
│   └── Dockerfile        # multi-stage build of the server binary
├── client-web/           # Next.js + Tailwind -- supervision UI
├── client-desktop/       # Tauri -- native app + tray + OS notifications
├── investigation/        # AI SRE agent (RAG / pgvector) -- extracted
├── .github/workflows/    # server + web + release CI (dormant, see Roadmap)
├── docker-compose.yml    # compose setup: server + db
├── Cargo.toml            # cargo workspace
└── package.json          # npm workspaces
```

### Development

```bash
# Server (Rust)
cd server
cargo run                                   # http://localhost:8080
cargo test                                  # unit + integration tests
cargo clippy --all-targets -- -D warnings   # lint
cargo fmt                                    # format

# Web client (Next.js, from the root via npm workspaces)
npm install
npm run dev --workspace client-web          # http://localhost:4242 (compose exposes 8081)
npm run build --workspace client-web
```

### Services

| Service                                                                                                                              | Stack        | Local address             |
| ------------------------------------------------------------------------------------------------------------------------------------ | ------------ | ------------------------- |
| `<img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/postgresql/postgresql-original.svg" height="18" />` `db`    | PostgreSQL   | `localhost:5432`        |
| `<img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/rust/rust-original.svg" height="18" />` `server`            | Rust / Axum  | `http://localhost:8080` |
| `<img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nextjs/nextjs-original.svg" height="18" />` `client_web`    | Next.js      | `http://localhost:4242` |
| `<img src="https://api.iconify.design/simple-icons/tauri.svg" height="18" />` `client_desktop`                                   | Tauri        | `:8081/client.AppImage` |
| `<img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/python/python-original.svg" height="18" />` `investigation` | AI SRE (RAG) | internal                  |

Cloud showcase (separate `opswarden-ops` repo):

<p>
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/kubernetes/kubernetes-plain.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/terraform/terraform-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/traefikproxy/traefikproxy-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/digitalocean/digitalocean-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/redis/redis-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/opentelemetry/opentelemetry-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/prometheus/prometheus-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/grafana/grafana-original.svg" height="25" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nixos/nixos-original.svg" height="25" />
</p>

## Architecture

Hexagonal dependency rule: **everything points inward.** The domain knows nothing
about Axum, SQLx, or the network.

```text
handlers (Axum, WS)  ->  app (use-cases)  ->  ports (traits)  ->  domain (pure)
                                                  ^
       adapters (Postgres, WS broadcaster, vault) implement the ports
```

- **Where business logic lives**: `server/src/domain` (models + invariants) and
  `server/src/app` (use-cases). Never in handlers or clients.
- **Where routes are wired**: `server/src/handlers` + `build_app()` in
  `server/src/lib.rs`.
- **Where persistence happens**: `server/src/adapters` (port implementations).
- **Where the WebSocket broadcaster lives**: an adapter implementing the
  `EventBus` port.

## Roadmap

**Foundations & rails**
- Scaffold monorepo: cargo workspace (`server`) + npm workspaces (`client-web`)
- Hexagonal skeleton `domain / ports / app / adapters / handlers` + `GET /health`
- Dynamic `/about.json` + SHA-256 `token` field (kickoff string)
- Green CI quality gate: `cargo fmt --check`, `clippy -D warnings`, ESLint, `prettier --check` pass on every push

**Real-time collaborative core**
- Email auth + JWT, `GET /me`, logout with token invalidation
- Teams + 3-role RBAC + invitation code + Manager transfer
- Incidents: open &rarr; acknowledged &rarr; escalated &rarr; resolved lifecycle + severities
- Real-time collaborative timeline (timestamped entries, Responder assignment)
- Core WebSockets: `incident_state_changed`, `incident_escalated`, `incident_assigned`, `timeline_entry_added`, `presence_update` + automatic client reconnection
- Postgres persistence (SQLx) + versioned migrations

**Automation & professionalization**
- Webhook receiver `POST /webhooks/{service}` + HMAC validation
- Hook engine (trigger + filters &rarr; reaction); 1 end-to-end rule: failing GitHub CI &rarr; `high` incident
- 1 external Action (GitHub) + 1 REAction (generic HTTP `Notify`, covers Slack)
- `/about.json` reflects the real catalog (nothing hard-coded client-side)
- WebSockets `rule_triggered`, `rule_failed`

**Desktop & delivery**
- Installable Tauri app (Linux / AppImage target) reusing the front-end
- Native OS notifications: assignment, `critical` severity, blocked Release + tray icon
- Final `docker-compose.yml`: `server` 8080 / `client_web` 8081 / `client_desktop` / `db`
- FR/EN i18n (labels, states, severities) persisted server-side

> CI (`.github/workflows/`) is intentionally **dormant** (gitignored) during the
> front/back rebuild. The workflows are ready and fixed; reactivate by removing
> the line from `.gitignore` based on project maturity.

## Contributing

Trunk-based workflow: short-lived branches (`feat/`, `fix/`, `chore/`, `docs/`,
`test/`), conventional commits, squash-merge into a protected `main`. Every PR
follows the [PR template](.github/pull_request_template.md), whose Definition of
Done requires: `clippy -D warnings` and `cargo fmt --check` green, `npm run lint`
- `format:check` + `typecheck` green, tests covering the happy path and at least
  one error path, business logic kept out of handlers and clients, impacted docs
  updated, and an atomic conventional commit.

## License

OpsWarden is distributed under the **Apache License 2.0**. See [LICENSE](LICENSE)
and [NOTICE](NOTICE).
