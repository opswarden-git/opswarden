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

# prépare les identités dédiées; le Manager conserve le vrai onboarding Team
demo-bootstrap:
    python3 tooling/demo.py bootstrap --target local

# peuple la Team de présentation avec le scénario déterministe à une Team
demo-presentation:
    python3 tooling/demo.py seed --target local --with-integrations
    python3 tooling/demo.py run --target local

# vérifie la configuration et les identités sans modifier de donnée
demo-doctor:
    python3 tooling/demo.py doctor --target local

# supprime uniquement les UUID et règles appartenant au scénario de présentation
demo-deseed:
    python3 tooling/demo.py deseed --target local

# ----- Server (Rust) -----

# serveur en mode développement.
# La stack compose publie déjà 8080 : si elle tourne, choisis un autre socket
# avec OPSWARDEN_BIND_ADDR (ex. OPSWARDEN_BIND_ADDR=0.0.0.0:8090 just dev).
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
#
# Une recette par job du gate, portant son nom, pour que la correspondance se
# lise sans ouvrir le workflow. Aucune ne masque un échec : pas de `|| true`.
#
# Un outil absent fait ÉCHOUER la recette au lieu de la sauter. Trois gates ont
# déjà rapporté un succès sans rien exécuter parce que `rg` manquait sur le
# runner ; un contrôle qui ne tourne pas ne doit jamais passer pour vert.
# `nix develop` fournit l'ensemble de la boîte à outils.

# Vérifie la présence des outils, sinon échoue en disant lesquels.
_require +tools:
    #!/usr/bin/env bash
    set -euo pipefail
    missing=()
    for tool in {{ tools }}; do command -v "$tool" >/dev/null || missing+=("$tool"); done
    if [ ${#missing[@]} -gt 0 ]; then
      echo "outils manquants : ${missing[*]}" >&2
      echo "entre dans le shell de dev : nix develop" >&2
      exit 1
    fi

# Base de comparaison des gates incrémentaux, comme BASE_SHA en CI.
_base:
    @git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD~1

# job "Workflow · Validate"
ci-workflow: (_require "actionlint" "zizmor" "shellcheck" "git" "docker")
    actionlint -color
    zizmor .github/workflows
    find tooling -type f -name '*.sh' -print0 | xargs -0 shellcheck
    ./tooling/check_source_hygiene.sh "$(just _base)"
    ./tooling/check_migration_policy.sh "$(just _base)"
    ./tooling/check_dockerfile_pins.sh
    docker compose config --quiet
    ./tooling/verify_release_version.sh

# job "Backend · Quality & security"
ci-backend-quality: (_require "cargo")
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo audit
    cargo audit --file client-desktop/src-tauri/Cargo.lock
    cargo deny check --config tooling/deny.toml

# Le build offline reproduit la CI (SQLX_OFFLINE) et attrape un cache .sqlx
# périmé AVANT le push -- sinon `cargo sqlx prepare` a été oublié et la CI casse.
# Prérequis : la DB compose tourne (`just up`).
# job "Backend · Build & test"
ci-backend-test: (_require "cargo" "sqlx")
    SQLX_OFFLINE=true cargo build --workspace
    cd server && sqlx migrate run
    cargo sqlx prepare --workspace --check -- --all-targets
    cargo test --workspace

# Lent : hors de `just ci`, présent dans `just ci-full`.
# job "Backend · Coverage"
ci-backend-coverage: (_require "cargo")
    cargo tarpaulin --config tooling/tarpaulin.toml
    ./tooling/summarize_source_coverage.sh

# job "Web · Quality & test"
ci-web: (_require "npm")
    npm run lint --workspace client-web
    npm run format:check --workspace client-web
    npm run typecheck --workspace client-web
    npm run test:coverage --workspace client-web
    ./tooling/audit_npm.sh

# job "Web · Build"
ci-web-build: (_require "npm")
    npm run build --workspace client-web

# Tous les fichiers de specs, comme la CI depuis #159. Prérequis : la pile
# tourne (`just up`) ; la recette attend qu'elle réponde avant de lancer.
# job "E2E · Browser suite"
ci-e2e: (_require "npm" "curl")
    #!/usr/bin/env bash
    set -euo pipefail
    probes=(
      "server|http://localhost:8080/health"
      "client_web|http://localhost:8081/en"
      "alertmanager|http://localhost:9093/-/ready"
    )
    down=()
    for _ in $(seq 1 60); do
      down=()
      for probe in "${probes[@]}"; do
        curl --fail --silent --output /dev/null "${probe#*|}" || down+=("${probe%%|*}")
      done
      if [ ${#down[@]} -eq 0 ]; then
        npm run test:e2e
        exit 0
      fi
      sleep 2
    done
    # Name the service rather than the stack. `docker compose up -d client_web`
    # does not pull alertmanager along, so it is the one that quietly drops out
    # and the failure would otherwise surface minutes later as "stack down".
    echo "service(s) injoignable(s) : ${down[*]}" >&2
    echo "lance 'just up' (qui amorce aussi la config alertmanager)" >&2
    exit 1

# Lent, et il lui faut le shell WebKit/GTK : `nix develop .#tauri`, pas le shell
# par défaut. La bibliothèque manquante n'est pas une commande, donc `_require`
# ne la verrait pas : on la sonde explicitement, sinon l'échec surgit au fond de
# pkg-config une fois le build lancé.
# job "Desktop (Linux) · Package"
ci-desktop: (_require "npm" "cargo" "pkg-config")
    #!/usr/bin/env bash
    set -euo pipefail
    pkg-config --exists webkit2gtk-4.1 || {
      echo "webkit2gtk-4.1 introuvable" >&2
      echo "entre dans le shell desktop : nix develop .#tauri" >&2
      exit 1
    }
    ./tooling/verify_desktop_asset_pins.sh
    npm run build --workspace client-desktop -- --bundles deb

# Ce que GitHub exécutera sur la PR, à la couverture et au paquet desktop près
# -- ces deux-là sont dans `ci-full`.
# Le gate, en local. À lancer avant chaque push.
ci: ci-workflow ci-backend-quality ci-backend-test ci-web ci-web-build ci-e2e

# `ci-desktop` exige `nix develop .#tauri` ; depuis le shell par défaut, lance
# `just ci ci-backend-coverage` puis le paquet à part.
# Le gate au complet, couverture et paquet desktop inclus.
ci-full: ci ci-backend-coverage ci-desktop

# Inventaire genere : 12 planches derivees du code (matchs exhaustifs,
# contrats testes, /about.json du serveur). Sortie dans tooling/inventory/dist.
inventory: (_require "node")
    node tooling/inventory/build.mjs

# Sert les planches pour visualisation/capture.
inventory-serve port="4300": inventory
    @echo "Inventaire sur http://localhost:{{port}}/index.html"
    python3 -m http.server {{port}} --directory tooling/inventory/dist

# Assemble le wiki public, les contrats canoniques et l'inventaire genere.
docs-build: (_require "docker") (_require "node")
    ./tooling/docs.sh

# Construit et sert le portail documentaire complet en local.
docs-serve port="8000": docs-build
    python3 -m http.server {{port}} --directory site
