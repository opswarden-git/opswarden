# Get started

## Requirements

The shortest path needs Docker with Compose. Native development additionally
needs Rust, Node.js and npm. Desktop development needs the
[Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your
operating system.

## Run the complete stack

Clone over HTTPS or SSH, then create the local environment file:

=== "HTTPS"

    ```bash
    git clone https://github.com/opswarden-git/opswarden.git
    cd opswarden
    cp .env.example .env
    ```

=== "SSH"

    ```bash
    git clone git@github.com:opswarden-git/opswarden.git
    cd opswarden
    cp .env.example .env
    ```

Review `OPSWARDEN_KICKOFF_TOKEN` and `DATABASE_URL` in `.env`, then start every
delivery service:

```bash
docker compose up --build
```

Compose starts PostgreSQL, the Rust server, a build-only desktop packager and
the Next.js client. The packager writes Linux packages to `./artifacts`.

| Service                   | Address                                             |
| ------------------------- | --------------------------------------------------- |
| Web application           | `http://localhost:8081/en` or `/fr`                 |
| REST and WebSocket server | `http://localhost:8080`                             |
| PostgreSQL                | `localhost:5433`                                    |
| Desktop downloads         | `/client.deb` and `/client.AppImage` on the web app |

If port `8081` is unavailable, expose another host port:

```bash
CLIENT_WEB_PORT=8091 docker compose up --build
```

## Verify the delivery

```bash
curl http://localhost:8080/health      # -> {"status":"ok"}
curl http://localhost:8080/about.json  # -> service catalogue + SHA-256 token
curl http://localhost:8081/en          # -> web UI; French lives at /fr
curl -I http://localhost:8081/client.deb
curl -I http://localhost:8081/client.AppImage
```

## Desktop application

The Tauri shell reuses the web UI and adds tray behavior plus native
notifications without bundling a second Chromium runtime. Run it against the
local web client during development:

```bash
just desktop-dev
```

Build and exercise the Linux delivery through Compose:

```bash
docker compose up --build
curl -I http://localhost:8081/client.deb
curl -I http://localhost:8081/client.AppImage
sudo apt install ./artifacts/OpsWarden_amd64.deb
./artifacts/client.AppImage
```

The AppImage smoke test is independently reproducible:

```bash
sh tooling/smoke_compose_appimage.sh
```

Tagged releases rebuild and publish the desktop artifacts through CI.

## Work on one part

=== "Server"

    ```bash
    cd server
    cargo run                                   # http://localhost:8080
    cargo test                                  # unit + integration tests
    cargo clippy --all-targets -- -D warnings   # lint
    cargo fmt                                   # format
    ```

=== "Web"

    Run these commands from the repository root:

    ```bash
    npm install
    npm run dev --workspace client-web           # http://localhost:4242
    npm run build --workspace client-web
    npm run lint --workspace client-web          # ESLint, blocking
    npm run format:check --workspace client-web  # Prettier, check only
    npm run typecheck --workspace client-web     # TypeScript, no emit
    npm run test --workspace client-web          # Vitest
    npm run test:coverage --workspace client-web # Vitest + V8 gate
    ```

=== "Desktop"

    ```bash
    just desktop-dev
    # or build Linux artifacts with Compose
    docker compose up --build client_desktop
    ```

## Coverage and quality gates

`just coverage` runs the complete Rust test suite through Tarpaulin while
reporting only runtime code under `server/src`. It excludes `main.rs` and test
functions, enforces 70% source-line coverage, and produces JSON, HTML, LCOV, XML
and a verified source-only summary. CI publishes the reports after every merge
to `main`.

The web gate uses the flat ESLint 9 configuration in
`client-web/eslint.config.mjs`, based on Next.js Core Web Vitals. Errors are
blocking, warnings remain visible, and generated `.next` and coverage output is
ignored. Vitest measures runtime source under `components`, `i18n`, `lib` and
`store`, excluding tests and type-only modules. CI enforces at least 70% line,
65% statement/function and 55% branch coverage.

## Before opening a pull request

Run the checks for the area you changed. The full command set, branch naming and
Definition of Done live in [Contributing](contributing/index.md). CI is the final
gate; local success is not a substitute for the protected branch checks.
