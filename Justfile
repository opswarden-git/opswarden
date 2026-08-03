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
up: seed-alertmanager
    docker compose up --build

# installe la configuration Alertmanager attendue par le service compose.
# Sans elle le conteneur sort en erreur 1 dès le démarrage : son fichier est
# monté depuis target/, que seul l'E2E remplissait. La CI fait déjà ce geste
# avant `docker compose up` ; cette recette le reproduit à l'identique pour
# que `just up` parte propre sur un clone neuf.
seed-alertmanager:
    #!/usr/bin/env bash
    set -euo pipefail

    dir=target/e2e-alertmanager
    config="$dir/alertmanager.yml"

    # Never overwrite a config the E2E run generated for a live connection.
    [[ -f "$config" ]] && exit 0

    # A plain `docker compose up` creates the bind-mount source as root when it
    # is missing, so the directory can already exist and be unwritable. Say so
    # instead of failing on a bare "Permission denied" from cp.
    if ! mkdir -p "$dir" 2>/dev/null \
      || ! install -m 0644 tooling/e2e/alertmanager-bootstrap.yml "$config" 2>/dev/null; then
        echo "Cannot write $config." >&2
        echo "A previous 'docker compose up' likely created $dir as root." >&2
        echo "Remove it, then retry:  sudo rm -rf $dir && just up" >&2
        exit 1
    fi

# arrête la stack
down:
    docker compose down

# reconstruit la stack et restaure une démo complète avec un Run d'automatisation
demo:
    docker compose up --build --detach --wait db server
    docker compose up --build --detach --wait --no-deps client_web
    ./tooling/seed_demo.sh
    ./tooling/test_github_webhook.sh

# remplit la base locale avec un dataset UX réaliste et rejouable
demo-seed:
    ./tooling/seed_demo.sh

# simule un workflow GitHub échoué avec une signature HMAC valide
demo-webhook:
    ./tooling/test_github_webhook.sh

# ----- Server (Rust) -----

# serveur en mode développement
dev:
    cargo run -p opswarden-server

# tests unit + intégration
test:
    cargo test --workspace

# campagne de mutations bornée au contrat de confiance Alertmanager
test-mutations-alertmanager:
    ./tooling/test_alertmanager_mutations.sh

# tests avec base de données éphémère (nettoyage garanti)
test-integration:
    #!/usr/bin/env bash
    set -euo pipefail

    root="$(git rev-parse --show-toplevel)"
    project="opswarden-integration-${USER:-user}-$$"
    temp_compose="$(mktemp)"

    cleanup() {
        docker compose \
          --project-name "$project" \
          -f "$temp_compose" \
          down -v --remove-orphans >/dev/null 2>&1 || true

        rm -f "$temp_compose"
    }

    trap cleanup EXIT INT TERM

    cat > "$temp_compose" <<'YAML'
    services:
      db:
        image: postgres:18.4-alpine3.24@sha256:9a8afca54e7861fd90fab5fdf4c42477a6b1cb7d293595148e674e0a3181de15
        environment:
          POSTGRES_USER: opswarden
          POSTGRES_PASSWORD: opswarden
          POSTGRES_DB: opswarden
        ports:
          - "127.0.0.1::5432"
        healthcheck:
          test: ["CMD-SHELL", "pg_isready -U opswarden"]
          interval: 10s
          timeout: 5s
          retries: 5
    YAML

    docker compose \
      --project-name "$project" \
      -f "$temp_compose" \
      up --detach --wait db

    db_port="$(
      docker compose \
        --project-name "$project" \
        -f "$temp_compose" \
        port db 5432 | cut -d: -f2
    )"

    export DATABASE_URL="postgres://opswarden:opswarden@127.0.0.1:${db_port}/opswarden"

    sqlx migrate run --source "$root/server/migrations"
    cargo test --manifest-path "$root/Cargo.toml" --workspace

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
    ./tooling/summarize_source_coverage.sh

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
    mkdir -p artifacts
    cargo modules dependencies -p opswarden-server --lib | dot -Tsvg > artifacts/modules.svg

# graphe des dépendances (nécessite cargo-depgraph + graphviz)
viz-deps:
    mkdir -p artifacts
    cargo depgraph | dot -Tsvg > artifacts/deps.svg

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

# parcours navigateur sur la stack locale ; le dataset démo est restauré après le run
web-e2e:
    npm run test:e2e

# ----- Repo -----

# prettier sur tout le repo (md, yaml, json, tsx…)
format:
    npx --yes prettier --write .

format-check:
    npx --yes prettier --check .

# prépare un commit de release et son tag local (ne pousse rien)
release version:
    ./tooling/prepare_release.sh {{version}}

# vérifie que la version est cohérente dans tous les manifestes et lockfiles
release-check:
    ./tooling/verify_release_version.sh

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
