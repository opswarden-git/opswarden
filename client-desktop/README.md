# OpsWarden Desktop

OpsWarden Desktop is a thin Tauri v2 shell around the existing web client. It
keeps one UI implementation and adds the operating-system capabilities that are
useful during incident response: native notifications and background presence
through a tray icon.

## Architecture

```text
Tauri webview ──▶ Next web client ── /api/* ──▶ Rust API
                       └──────── WebSocket ───▶ Rust API
```

The shell operates in URL mode; it does not bundle a second copy of the Next
application:

- development loads `http://localhost:4242`;
- packaged builds load `http://localhost:8081`;
- the web client proxies HTTP API calls and connects to the realtime WebSocket.

Consequently, an installed desktop package still needs a reachable OpsWarden
web deployment. It is not an offline or self-contained distribution.

## Native capabilities

The current shell provides:

- a launch notification to prove native notification delivery;
- live notifications for incidents assigned to the current user;
- live notifications for high or critical incident escalations;
- live notifications when an active incident blocks a release;
- a tray icon with **Show** and **Quit** actions;
- hide-to-tray when the main window is closed;
- restoration and focus from the tray icon.

Notification permission is requested lazily by the web client. In a normal
browser, the same notification adapter is a no-op.

## Development

A real Wayland or X session is required to display and validate the window.

```bash
# From the repository root. Reuses an existing web server on :4242 or starts it.
just desktop-dev

# Equivalent direct entrypoint.
./client-desktop/dev.sh
```

The dedicated Nix shell contains the WebKit/GTK toolchain:

```bash
nix develop .#tauri
```

## Build and distribution

The local Compose build produces a Debian package in `./artifacts` and exposes
it from the running web client at `http://localhost:8081/client.deb`:

```bash
just demo
```

Release CI builds an AppImage on Ubuntu and attaches it to tagged GitHub
releases. Building only the linked Rust binary, without packaging, remains
useful as a headless compile check:

```bash
nix develop .#tauri --command bash -lc \
  'cd client-desktop/src-tauri && cargo build'
```

## Manual display checks

Run these checks in a real desktop session:

1. the login page renders and authentication stays inside the webview;
2. the launch notification appears;
3. closing the window hides it without ending the process;
4. clicking **Show** or the tray icon restores and focuses the window;
5. **Quit** terminates the process;
6. assignment, escalation and blocked-release events produce notifications.

## Current limits

- no offline mode or bundled Next server;
- no auto-updater;
- no desktop-specific redesign;
- Linux packaging is the currently exercised distribution path;
- visual behavior and notification delivery still depend on the host desktop,
  notification daemon and granted permissions.
