#!/usr/bin/env bash
# Launch the OpsWarden desktop shell in dev mode on NixOS.
#
# DISPLAY-REQUIRED: this opens a real window and needs a Wayland/X session
# (e.g. Hyprland). On a headless machine it will not render. Runtime env mirrors
# the proven hello-world baseline (docs/code/05-...).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

cd "$ROOT"
exec nix develop .#tauri --command bash -c '
  # SSL certs for any outbound HTTPS from the webview/runtime.
  export SSL_CERT_FILE="${SSL_CERT_FILE:-/etc/ssl/certs/ca-bundle.crt}"
  [ -f "$SSL_CERT_FILE" ] || export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
  # WebKit-on-Wayland: avoid the blank-window DMABUF path.
  export WEBKIT_DISABLE_DMABUF_RENDERER=1
  export GDK_BACKEND="${GDK_BACKEND:-wayland}"
  export GSETTINGS_BACKEND=memory
  # beforeDevCommand (in tauri.conf.json) reuses an existing :4242 dev server,
  # or starts client-web if none is running (avoids EADDRINUSE).
  cd client-desktop && npm run tauri dev
'
