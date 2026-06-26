# --- client-desktop/Dockerfile ---
#
# Builds an OpsWarden desktop Linux package in an Ubuntu 22.04 (FHS) environment —
# the path Tauri's bundler expects, which it cannot take on the NixOS host (the
# bundler resolves libayatana-appindicator through Nix linker flags, see U1).
# ubuntu:22.04 (glibc 2.35) matches the platform `tauri-action` uses in CI.
#
# Local default = the `.deb` bundle: it builds reliably here and is a real
# installable Linux package (`apt install ./OpsWarden.deb`). The **AppImage** is
# deliberately left to CI/U4 (release.yml `tauri-action` on a GitHub ubuntu-22.04
# runner via `--bundles appimage`): Tauri's pinned `linuxdeploy` v1-alpha fails its self-
# recursive AppImage invocation inside a local Docker container (`subprocess
# failed (exit code 6)`, identical on Debian 12 and Ubuntu 22.04) — a documented
# linuxdeploy/Docker fragility, not a config gap. The AppImage tooling
# (libfuse2 …) stays installed so the same recipe can `--bundles appimage` on a
# runner where linuxdeploy works.
#
# The shell is URL-mode (loads http://localhost:8081), so no web frontend is
# built here. The build-only service copies the artifact to a /out bind mount.

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
# Tauri v2 Linux toolchain (matches the tauri-action recipe) + AppImage tooling
# (kept for the CI `--bundles appimage` path).
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl wget ca-certificates build-essential pkg-config libssl-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
    patchelf file desktop-file-utils libfuse2 \
    && rm -rf /var/lib/apt/lists/*

# Rust (stable) + Node 20 (NodeSource).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
# Use the root lockfile so the Tauri CLI version is deterministic. URL-mode
# desktop packaging does not build the web workspace.
COPY package.json package-lock.json ./
COPY client-desktop/package.json ./client-desktop/package.json
RUN npm ci --workspace client-desktop --include-workspace-root=false --no-audit --no-fund
COPY client-desktop/ ./client-desktop/
WORKDIR /app/client-desktop

# Compile + bundle the `.deb`, then deposit it on the /out bind mount. NO_STRIP
# keeps patchelf from choking. (`--bundles appimage` is the CI override.)
ENV NO_STRIP=true
CMD ["sh", "-c", "npm run tauri build -- --bundles deb && mkdir -p /out && cp -v src-tauri/target/release/bundle/deb/*.deb /out/OpsWarden_amd64.deb && echo 'Desktop .deb ready at /out/OpsWarden_amd64.deb'"]
