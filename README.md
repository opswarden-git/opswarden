<div align="center">
  <img src="https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/opswarden-ops/heroicon.png" alt="OpsWarden" width="120" />
  <h1>OpsWarden</h1>
  <p>
    <a href="https://github.com/opswarden-git/opswarden/actions/workflows/ci.yml"><img src="https://github.com/opswarden-git/opswarden/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <img src="https://img.shields.io/github/v/release/opswarden-git/opswarden?style=flat" alt="Release" />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue?style=flat" alt="License: Apache 2.0" /></a>
    <img src="https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white" alt="Rust" />
    <img src="https://img.shields.io/badge/Next.js-black?style=flat&logo=next.js&logoColor=white" alt="Next.js" />
    <img src="https://img.shields.io/badge/Tauri-FFC131?style=flat&logo=tauri&logoColor=white" alt="Tauri" />
    <img src="https://img.shields.io/badge/PostgreSQL-316192?style=flat&logo=postgresql&logoColor=white" alt="PostgreSQL" />
  </p>
</div>


## What is OpsWarden?



**OpsWarden** is a real-time incident response and release coordination platform
for technical teams. Incidents are triaged and resolved in one shared workspace;
releases are validated step by step and automatically blocked when an active
incident makes further deployment unsafe.

External events can trigger internal actions through an
**Action&rarr;REAction** rule engine. The current implementation demonstrates the
complete path from a signed GitHub CI failure webhook to a new incident.

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

The tested alpha ships as a Next.js web app and an installable Tauri desktop
client backed by a Rust/Axum server and PostgreSQL. Rust keeps lifecycle rules
strongly typed, PostgreSQL protects concurrent multi-user state, and Tauri adds
native desktop behavior without introducing a second application architecture.

## For developers

OpsWarden is built to be run both locally for development and in the cloud for production. To get a feel for the platform on your own machine, you can launch the entire stack in just a few commands using Docker:

```bash
git clone https://github.com/opswarden-git/opswarden.git
cd opswarden
cp .env.example .env
docker compose up --build
```

Once the containers are up, open `http://localhost:8081/en` (`/fr` for French). Here is a breakdown of the services that are running:

| Icon                                                                                                                         | Service          | Stack       | Local address                   |
| ---------------------------------------------------------------------------------------------------------------------------- | ---------------- | ----------- | ------------------------------- |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/postgresql/postgresql-original.svg" width="18" alt="" /> | `db`             | PostgreSQL  | `localhost:5433`                |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/rust/rust-original.svg" width="18" alt="" />             | `server`         | Rust / Axum | `http://localhost:8080`         |
| <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nextjs/nextjs-original.svg" width="18" alt="" />         | `client_web`     | Next.js     | `:4242` dev / `:8081` Compose   |
| <img src="https://api.iconify.design/simple-icons/tauri.svg" width="18" alt="" />                                            | `client_desktop` | Tauri       | URL mode via `just desktop-dev` |

For a deep dive into setup, configuration, and advanced development commands, be sure to check out our comprehensive **[Technical documentation](https://opswarden-git.github.io/opswarden/)**. It includes everything from architectural decisions and our [WebSocket protocol](https://opswarden-git.github.io/opswarden/reference/websocket/) to the complete [REST API](https://opswarden-git.github.io/opswarden/reference/rest-api/) and [data model](https://opswarden-git.github.io/opswarden/reference/data-model/).

<p>
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/rust/rust-original.svg" height="25" alt="Rust" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/postgresql/postgresql-original.svg" height="25" alt="PostgreSQL" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nextjs/nextjs-original.svg" height="25" alt="Next.js" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/typescript/typescript-original.svg" height="25" alt="TypeScript" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/tailwindcss/tailwindcss-original.svg" height="25" alt="Tailwind CSS" />
  <img src="https://api.iconify.design/simple-icons/tauri.svg" height="25" alt="Tauri" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/docker/docker-original.svg" height="25" alt="Docker" />
  <img src="https://api.iconify.design/simple-icons/githubactions.svg" height="25" alt="GitHub Actions" />
</p>

When it comes to production deployment, we take infrastructure seriously. The **[`opswarden-ops`](https://github.com/opswarden-git/opswarden-ops)** repository houses all of our cloud and observability engineering. We rely on modern tooling to keep the platform reliable and observable:

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

Finally, if you're looking for our public-facing presentation, you can find the Next.js source code for our landing page in the **[`opswarden-website`](https://github.com/opswarden-git/opswarden-website)** repository.

<p>
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/nextjs/nextjs-original.svg" height="25" alt="Next.js" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/react/react-original.svg" height="25" alt="React" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/typescript/typescript-original.svg" height="25" alt="TypeScript" />
  <img src="https://cdn.jsdelivr.net/gh/devicons/devicon@latest/icons/tailwindcss/tailwindcss-original.svg" height="25" alt="Tailwind CSS" />
  <img src="https://api.iconify.design/simple-icons/vercel.svg" height="25" alt="Vercel" />
  <img src="https://api.iconify.design/simple-icons/githubactions.svg" height="25" alt="GitHub Actions" />
</p>

### Contributing

Use short-lived `feat/`, `fix/`, `chore/`, `docs/` or `test/` branches and
conventional commits. Pull requests are squash-merged into protected `main` and
must satisfy the [Definition of Done](.github/pull_request_template.md).

### License

OpsWarden is distributed under the **Apache License 2.0**. See [LICENSE](LICENSE)
and [NOTICE](NOTICE).
