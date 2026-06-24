set shell := ["bash", "-cu"]

# Default DB for the test / coverage / ci-build-test recipes. The PG adapter
# tests use #[sqlx::test], which needs DATABASE_URL and a Postgres role with
# CREATEDB (it spins an ephemeral database per test). An already-exported
# DATABASE_URL is respected; otherwise this points at the compose DB.
export DATABASE_URL := env_var_or_default("DATABASE_URL", "postgres://opswarden:opswarden@localhost:5433/opswarden")

# liste les recettes disponibles
default:
    @just --list

# ----- App (docker compose) -----

# lance server + db (contrat jury)
up:
    docker compose up --build

# arrête la stack
down:
    docker compose down

# ----- Server (Rust) -----

# serveur en mode développement
dev:
    cargo run -p opswarden-server

# tests unit + intégration
test:
    cargo test --workspace

# vérification rapide (sans build complet)
check:
    cargo check --workspace --all-targets

# lint (warnings = erreurs)
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# format
fmt:
    cargo fmt --all

# format check (ce que vérifie la CI)
fmt-check:
    cargo fmt --all --check

# couverture de code (nécessite cargo-tarpaulin)
coverage:
    cargo tarpaulin --config tooling/tarpaulin.toml

# audit supply-chain (nécessite cargo-deny / cargo-audit / cargo-udeps)
audit:
    cargo deny check --config tooling/deny.toml || true
    cargo audit || true
    RUSTC_BOOTSTRAP=1 cargo udeps --workspace --all-targets || true

# profilage CPU (nécessite cargo-flamegraph)
flamegraph:
    cargo flamegraph -p opswarden-server

# graphe des modules (nécessite cargo-modules + graphviz)
viz-modules:
    cargo modules dependencies -p opswarden-server --lib | dot -Tsvg > docs/modules.svg

# graphe des dépendances (nécessite cargo-depgraph + graphviz)
viz-deps:
    cargo depgraph | dot -Tsvg > docs/deps.svg

# ----- Web (Next.js) -----

# client web en dev
web-dev:
    npm run dev --workspace client-web

# client desktop (Tauri) en dev -- display requis (Wayland/X, ex. Hyprland)
desktop-dev:
    ./client-desktop/dev.sh

# qualité côté web (lint + format + types)
web-check:
    npm run lint --workspace client-web
    npm run format:check --workspace client-web
    npm run typecheck --workspace client-web

# ----- Repo -----

# prettier sur tout le repo (md, yaml, json, tsx…)
format:
    npx --yes prettier --write .

format-check:
    npx --yes prettier --check .

# rapport de santé (tokei + deny + audit)
health:
    @mkdir -p tooling
    @echo "# OpsWarden — health report ($(date '+%Y-%m-%d %H:%M'))" > tooling/health_report.md
    @echo '## Code stats (tokei)' >> tooling/health_report.md
    @echo '```' >> tooling/health_report.md
    @tokei --exclude target --exclude node_modules >> tooling/health_report.md 2>/dev/null || true
    @echo '```' >> tooling/health_report.md
    @echo '## Supply chain (cargo deny)' >> tooling/health_report.md
    @echo '```' >> tooling/health_report.md
    @cargo deny check --config tooling/deny.toml >> tooling/health_report.md 2>&1 || true
    @echo '```' >> tooling/health_report.md
    @echo "Report written to tooling/health_report.md"

# ----- CI locale (miroir de .github/workflows/ci.yml) -----

# job "checks" : fmt + clippy (--all-features) + supply-chain, STRICT comme la CI
# (pas de `|| true` : un échec ici = un échec en CI).
ci-checks:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo audit
    cargo deny check --config tooling/deny.toml

# job "build-test" : build offline (valide que le cache .sqlx colle au code) puis
# migrations + tests sur la vraie DB. Prérequis : la DB compose tourne (`just up`).
# Le build offline reproduit la CI (SQLX_OFFLINE) et attrape un cache .sqlx périmé
# AVANT le push -- sinon `cargo sqlx prepare` a été oublié et la CI casse.
ci-build-test:
    SQLX_OFFLINE=true cargo build --workspace
    cd server && sqlx migrate run
    cargo test --workspace

# pipeline complète : ce que GitHub exécutera sur la PR. À lancer avant chaque push.
ci: ci-checks ci-build-test
