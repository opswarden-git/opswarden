# Get started

## Requirements

The shortest path needs Docker with Compose. Native development additionally
needs Rust, Node.js and npm; desktop development needs the Tauri system
dependencies for your operating system.

## Run the complete stack

```bash
git clone https://github.com/opswarden-git/opswarden.git
cd opswarden
cp .env.example .env
docker compose up --build
```

Compose starts PostgreSQL, the Rust server, a build-only desktop packager and
the Next.js client.

| Service                   | Address                                             |
| ------------------------- | --------------------------------------------------- |
| Web application           | `http://localhost:8081/en` or `/fr`                 |
| REST and WebSocket server | `http://localhost:8080`                             |
| PostgreSQL                | `localhost:5433`                                    |
| Desktop downloads         | `/client.deb` and `/client.AppImage` on the web app |

Verify the public surface:

```bash
curl http://localhost:8080/health
curl http://localhost:8080/about.json?locale=en
curl -I http://localhost:8081/en
```

## Work on one part

=== "Server"

    ```bash
    cd server
    cargo run
    cargo test
    cargo clippy --all-targets -- -D warnings
    ```

=== "Web"

    ```bash
    npm install
    npm run dev --workspace client-web
    npm run test --workspace client-web
    npm run typecheck --workspace client-web
    ```

=== "Desktop"

    ```bash
    just desktop-dev
    # or build Linux artifacts with Compose
    docker compose up --build client_desktop
    ```

## Before opening a pull request

Run the checks for the area you changed. The full command set, branch naming and
Definition of Done live in [Contributing](contributing/index.md). CI is the final
gate; local success is not a substitute for the protected branch checks.
