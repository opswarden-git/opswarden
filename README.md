<p align="center">
  <img src="client-web/public/assets/heroicon.png" alt="OpsWarden" width="130" />
  <h1 align="center">OpsWarden</h1>
</p>

<p align="center">
  <a href="https://github.com/opswarden-git/opswarden/actions/workflows/ci.yml"><img src="https://github.com/opswarden-git/opswarden/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="https://github.com/opswarden-git/opswarden/actions/workflows/release.yml"><img src="https://github.com/opswarden-git/opswarden/actions/workflows/release.yml/badge.svg" alt="Release workflow" /></a>
  <a href="https://opswarden-git.github.io/opswarden/"><img src="https://img.shields.io/badge/docs-GitHub_Pages-F4C430?style=flat-square&logo=materialformkdocs&logoColor=000000" alt="Technical documentation" /></a>
  <a href="https://github.com/opswarden-git/opswarden/releases/latest"><img src="https://img.shields.io/github/v/release/opswarden-git/opswarden?style=flat-square&color=F4C430&label=release" alt="Latest release" /></a>
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

<p align="center">
  <a href="#what-is-opswarden">What is OpsWarden?</a> ·
  <a href="#getting-started">Getting started</a> ·
  <a href="https://opswarden-git.github.io/opswarden/">Documentation</a> ·
  <a href="https://opswarden-git.github.io/opswarden/reference/rest-api/">REST API</a> ·
  <a href="#contributing">Contributing</a> ·
  <a href="#license">License</a>
</p>

## What is OpsWarden?

**OpsWarden** is a real-time incident response and release coordination platform
for technical teams. Incidents are triaged and resolved in one shared workspace;
releases are validated step by step and automatically blocked when an active
incident makes further deployment unsafe.

External events can trigger internal actions through an
**Action&rarr;REAction** rule engine. The current implementation demonstrates the
complete path from a signed GitHub CI failure webhook to a new incident.

The tested alpha ships as a Next.js web app and an installable Tauri desktop
client backed by a Rust/Axum server and PostgreSQL. Rust keeps lifecycle rules
strongly typed, PostgreSQL protects concurrent multi-user state, and Tauri adds
native desktop behavior without introducing a second application architecture.

### Incident response

OpsWarden gives responders one shared operational record for an incident: its
severity and lifecycle, current owner, live participant presence, editable
timeline, emoji reactions and activity history. Updates are persisted in
PostgreSQL and broadcast over WebSocket so the web and desktop clients converge
without moving business rules out of the Rust server. PostgreSQL was preferred
to SQLite because concurrent team writes, foreign keys and transactional
lifecycle invariants are central to this multi-user server.

<p align="center">
  <a href="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/incidents.png">
    <img src="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/incidents.png" alt="OpsWarden incident queue" width="900" />
  </a>
</p>

### Safe release coordination

Release coordination turns a deployment into an ordered, accountable sequence:
responders validate each step, progress remains visible to the team, and linked
active incidents automatically block unsafe advancement until they are resolved.
This keeps release state and operational risk in the same workspace.

<p align="center">
  <a href="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/releases.png">
    <img src="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/releases.png" alt="OpsWarden release coordination" width="900" />
  </a>
</p>

### Team operations

Teams are the security and collaboration boundary: membership, invitations,
presence and Observer/Responder/Manager permissions govern every operation.
Managers can transfer ownership, moderate or ban members, configure encrypted
GitHub and HTTP integrations, and create Action&rarr;REAction rules; teammates can
also exchange private messages without leaving their shared operational context.

<p align="center">
  <a href="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/teams.png">
    <img src="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/teams.png" alt="OpsWarden team management" width="900" />
  </a>
</p>

## Getting started

```bash
git clone https://github.com/opswarden-git/opswarden.git
cd opswarden
cp .env.example .env
docker compose up --build
```

Open `http://localhost:8081/en` (`/fr` for French). The complete setup,
configuration, desktop installation and development commands live in the
**[Getting started guide](https://opswarden-git.github.io/opswarden/getting-started/)**.

### Services

| Icon                                                                                                                         | Service          | Stack       | Local address                   |
| ---------------------------------------------------------------------------------------------------------------------------- | ---------------- | ----------- | ------------------------------- |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/postgresql/postgresql-original.svg" width="18" alt="" /> | `db`             | PostgreSQL  | `localhost:5433`                |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/rust/rust-original.svg" width="18" alt="" />             | `server`         | Rust / Axum | `http://localhost:8080`         |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nextjs/nextjs-original.svg" width="18" alt="" />         | `client_web`     | Next.js     | `:4242` dev / `:8081` Compose   |
| <img src="https://api.iconify.design/simple-icons/tauri.svg" width="18" alt="" />                                            | `client_desktop` | Tauri       | URL mode via `just desktop-dev` |

## Also see

- **[Technical documentation](https://opswarden-git.github.io/opswarden/)** —
  searchable implementation guides covering the
  [architecture](https://opswarden-git.github.io/opswarden/architecture/),
  [REST API](https://opswarden-git.github.io/opswarden/reference/rest-api/),
  [data model](https://opswarden-git.github.io/opswarden/reference/data-model/),
  [WebSocket protocol](https://opswarden-git.github.io/opswarden/reference/websocket/)
  and [UI guidelines](https://opswarden-git.github.io/opswarden/design/ui-guidelines/).
- **[`opswarden-ops`](https://github.com/opswarden-git/opswarden-ops)** — cloud,
  deployment and observability engineering for OpsWarden.
- **[`opswarden-website`](https://github.com/opswarden-git/opswarden-website)** —
  the public marketing website.

<p>
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/kubernetes/kubernetes-plain.svg" height="25" alt="Kubernetes" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/terraform/terraform-original.svg" height="25" alt="Terraform" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/traefikproxy/traefikproxy-original.svg" height="25" alt="Traefik Proxy" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/digitalocean/digitalocean-original.svg" height="25" alt="DigitalOcean" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/redis/redis-original.svg" height="25" alt="Redis" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/opentelemetry/opentelemetry-original.svg" height="25" alt="OpenTelemetry" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/prometheus/prometheus-original.svg" height="25" alt="Prometheus" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/grafana/grafana-original.svg" height="25" alt="Grafana" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nixos/nixos-original.svg" height="25" alt="NixOS" />
</p>

## Contributing

Use short-lived `feat/`, `fix/`, `chore/`, `docs/` or `test/` branches and
conventional commits. Pull requests are squash-merged into protected `main` and
must satisfy the [Definition of Done](.github/pull_request_template.md).

## License

OpsWarden is distributed under the **Apache License 2.0**. See [LICENSE](LICENSE)
and [NOTICE](NOTICE).
