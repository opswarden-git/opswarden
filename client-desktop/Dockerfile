# --- client-desktop/Dockerfile ---
#
# Builds an OpsWarden desktop Linux package in an Ubuntu 22.04 (FHS) environment —
# the path Tauri's bundler expects, which it cannot take on the NixOS host (the
# bundler resolves libayatana-appindicator through Nix linker flags, see U1).
# ubuntu:22.04 (glibc 2.35) matches the platform `tauri-action` uses in CI.
#
# Both the `.deb` and canonical AppImage are built here. AppImage helper tools
# run in extract-and-run mode because a build container has no FUSE device.
# Tauri patches the main linuxdeploy helper for this mode but not its nested
# appimage output plugin, so that plugin is preseeded and its three AppImage
# signature bytes are cleared. This changes only the build tool's ELF launch
# metadata; the produced client.AppImage keeps its required Type 2 signature.
#
# The shell is URL-mode (loads http://localhost:8081), so no web frontend is
# built here. The build-only service copies the artifact to a /out bind mount.

FROM ubuntu:22.04@sha256:0e0a0fc6d18feda9db1590da249ac93e8d5abfea8f4c3c0c849ce512b5ef8982

ENV DEBIAN_FRONTEND=noninteractive
# Tauri v2 Linux toolchain (matches the tauri-action recipe) + AppImage tooling
# (kept for the CI `--bundles appimage` path).
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl wget ca-certificates build-essential pkg-config libssl-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
    patchelf file desktop-file-utils libfuse2 xvfb xauth \
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

# Preseed every Tauri AppImage helper so Compose does not depend on several
# late GitHub downloads after the Rust build has already completed. Tauri
# reuses these exact cache names.
RUN mkdir -p /root/.cache/tauri \
    && curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
      "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64?download=1" \
      -o /root/.cache/tauri/AppRun-x86_64 \
    && curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
      "https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy/linuxdeploy-x86_64.AppImage?download=1" \
      -o /root/.cache/tauri/linuxdeploy-x86_64.AppImage \
    && curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
      https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/b5eb8d05b4c0ed40107fe2158c5d8527f94568ef/linuxdeploy-plugin-gtk.sh \
      -o /root/.cache/tauri/linuxdeploy-plugin-gtk.sh \
    && curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
      https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gstreamer/2a2e67491c32995a3f279ad0ecbe77abd512b42a/linuxdeploy-plugin-gstreamer.sh \
      -o /root/.cache/tauri/linuxdeploy-plugin-gstreamer.sh \
    # Pin an immutable tagged release, never the project's `continuous` tag:
    # that tag is rolling and its assets were rewritten upstream on 2026-08-01,
    # which broke the digest below and made `just up` fail on every fresh clone.
    && curl -fL --retry 5 --retry-delay 2 --retry-all-errors -sS \
      "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/1-alpha-20250213-1/linuxdeploy-plugin-appimage-x86_64.AppImage" \
      -o /root/.cache/tauri/linuxdeploy-plugin-appimage.AppImage \
    && printf '%s\n' \
      'f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f  /root/.cache/tauri/AppRun-x86_64' \
      'e762bea85c8eb0d4b3508d46e5c1f037f717d0f9303ae3b4aafc8b04991fa1ef  /root/.cache/tauri/linuxdeploy-x86_64.AppImage' \
      'cb379f9b0733e9ad9f8bd78f8c2fa038aef2478523bb7d4c8e64ff6a1ea3501a  /root/.cache/tauri/linuxdeploy-plugin-gtk.sh' \
      'c107b49d84edbffc6ab226ed1007e0626a4f7aa2c3a36b7782bef62351d49e94  /root/.cache/tauri/linuxdeploy-plugin-gstreamer.sh' \
      '992d502a248e14ab185448ddf6f6e7d25558cb84d4623c354c3af350c25fccb3  /root/.cache/tauri/linuxdeploy-plugin-appimage.AppImage' \
      | sha256sum -c - \
    && chmod 755 /root/.cache/tauri/* \
    && dd if=/dev/zero bs=1 count=3 seek=8 conv=notrunc \
      of=/root/.cache/tauri/linuxdeploy-x86_64.AppImage \
    && dd if=/dev/zero bs=1 count=3 seek=8 conv=notrunc \
      of=/root/.cache/tauri/linuxdeploy-plugin-appimage.AppImage

# Compile, bundle and smoke-test both Linux deliverables before depositing them
# on the /out bind mount. NO_STRIP keeps patchelf from choking.
ENV NO_STRIP=true
ENV APPIMAGE_EXTRACT_AND_RUN=1
CMD ["sh", "scripts/build-linux-artifacts.sh"]
