# OpsWarden Desktop (Tauri v2) — N1 spike

A **thin native shell** around the existing OpsWarden web client. It does not
reimplement any UI: it opens a WebKit window pointed at the running Next app and
adds native-OS capabilities (notifications, later a tray). This is the N1
**capability spike** — proven to build, not yet visually validated.

## Architecture (N1: URL mode)

```
Tauri window ── loads ──▶ http://localhost:4242  (Next dev/start, client-web)
                                   └── /api/* proxied ──▶ http://localhost:8080 (Rust server)
                                   └── ws://localhost:8080/ws (realtime)
```

- The desktop **loads a URL**; it does **not** bundle the web app.
  `frontendDist` in `tauri.conf.json` is a URL, so the produced binary is
  **not self-contained** — it needs the web client reachable at `:4242`.
- A self-contained, installable artifact (AppImage/deb that bundles the UI)
  requires Next **static export**, which `client-web` cannot do as-is
  (`next-intl` `[locale]` routing + the `/api/*` rewrite-proxy). That is a
  deliberate **out-of-scope** decision for N1 — see "Next steps".

## Prerequisites (NixOS)

The WebKit/GTK toolchain lives in a dedicated Nix dev shell:

```bash
nix develop .#tauri      # from the repo root (opswarden-app/)
```

Running the **window** additionally needs a real Wayland/X session (e.g.
Hyprland). A headless machine can build the binary but cannot display it.

## Commands

```bash
# Dev (display-required): reuses/starts client-web on :4242, then opens the window.
./client-desktop/dev.sh
#   └ equivalently, inside `nix develop .#tauri`:
#     cd client-desktop && npm run tauri dev

# Build the desktop binary (compiles + links WebKit; headless-OK):
nix develop .#tauri --command bash -c 'cd client-desktop/src-tauri && cargo build'
```

> `npm run tauri build` (deb/AppImage packaging) is intentionally **not** part of
> N1 — see the static-export caveat above.

## What N1 proves (headless, on this machine)

- `nix develop .#tauri` resolves `webkit2gtk-4.1`, `javascriptcoregtk-4.1`,
  `libsoup-3.0`, `gtk+-3.0` via `pkg-config`.
- `client-desktop/src-tauri` **compiles and links** against WebKit/GTK
  (`cargo build` → `opswarden-desktop` binary).
- `tauri.conf.json` validates and `generate_context!` embeds the icon set.
- `tauri-plugin-notification` compiles and is registered.

## What still needs a real display (Romeo / Hyprland)

- The window actually **renders** `http://localhost:4242`.
- The **launch notification** ("Desktop shell connected.") appears.
- Window controls / future tray behavior.

Use `./client-desktop/dev.sh` on a Wayland session to verify these.

## Non-goals (N1)

No release lifecycle, no blocked-release notification, no PM/moderation, no
offline support, no auto-updater, no desktop-specific redesign, no static-export
refactor, no AppImage/release claim, no `ws.ts` event hook (the launch
notification is fired from Rust `setup()` as a pure capability proof).

## Next steps (beyond N1)

1. Verify launch + notification on a Wayland session (display gate).
2. Tie a notification to a live event (`incident_assigned`, critical
   `incident_escalated`) — either a guarded `client-web/lib/ws.ts` call when
   `window.__TAURI__` is present, or a desktop-side WS listener.
3. Add the tray icon (VIGIL background-presence requirement).
4. Decide the build/installable strategy (static export vs. deployed URL vs.
   bundled `next start`) before promising AppImage + CI artifact.
