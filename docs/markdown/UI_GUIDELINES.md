# OpsWarden UI/UX Guidelines

This document describes the UI contract currently implemented in the web client.
The code is the source of truth:

- `client-web/app/globals.css` defines product tokens and reusable utility classes.
- `client-web/tailwind.config.ts` exposes those tokens to Tailwind.
- `client-web/app/[locale]/layout.tsx` defines the font stack.
- `client-web/components/incidents/SeverityChip.tsx` and
  `client-web/components/incidents/StateChip.tsx` define incident status visuals.

Status: web client only. The desktop client is not present in the repository yet;
when it is added, it should reuse the same visual rules.

## 1. Product Direction

OpsWarden uses a dark operational-control-room interface: dense, readable, and
work-focused. The UI favors fast scanning, direct actions, and visible incident
state over decorative presentation.

The current visual language uses:

- opaque dark surfaces over a subtle dotted background;
- gold for primary actions and active navigation;
- semantic colors for incident severity and lifecycle state;
- compact rows, chips, and command panels.

## 2. Typography

Implemented fonts:

| Role           | Font           | Source                               |
| -------------- | -------------- | ------------------------------------ |
| Main UI        | Inter          | `client-web/app/[locale]/layout.tsx` |
| Technical data | JetBrains Mono | `client-web/app/[locale]/layout.tsx` |

Usage in the codebase:

- main headings: bold sans-serif, usually `text-2xl` or `text-lg`;
- row metadata, IDs, and technical values: `font-mono`;
- operational labels and table text: compact `text-xs` to `text-sm`.

## 3. Color Tokens

The implemented palette lives in `client-web/app/globals.css`.

| Token          | Value                    | Usage                                    |
| -------------- | ------------------------ | ---------------------------------------- |
| `--bg`         | `#15161a`                | application background                   |
| `--panel`      | `#1b1c20`                | main surfaces via `.surface` / `.glass`  |
| `--panel-2`    | `#212228`                | secondary surfaces via `.surface-subtle` |
| `--ow-border`  | `rgba(255,255,255,0.08)` | default border                           |
| `--text`       | `#e7e7ea`                | primary text                             |
| `--ow-muted`   | `#989ba1`                | secondary text                           |
| `--ow-muted-2` | `#6f737a`                | quieter text / placeholders              |
| `--gold`       | `#fbc02d`                | primary action / active UI               |
| `--gold-hover` | `#f9a825`                | primary hover                            |
| `--gold-ink`   | `#1a1405`                | text on gold buttons                     |

Destructive actions currently use `.ow-danger`:

| Token/class        | Value     | Usage               |
| ------------------ | --------- | ------------------- |
| `.ow-danger`       | `#ff2d2d` | destructive buttons |
| `.ow-danger:hover` | `#e91919` | destructive hover   |

## 4. Incident Visual Mapping

### Severity

Implemented in `SeverityChip`.

| Severity   | Token                     | Visual cue                                       |
| ---------- | ------------------------- | ------------------------------------------------ |
| `low`      | `--sev-low: #3b82f6`      | `AlertCircle` icon + localized text              |
| `medium`   | `--sev-medium: #f59e0b`   | `AlertTriangle` icon + localized text            |
| `high`     | `--sev-high: #fb7d3c`     | `AlertOctagon` icon + localized text             |
| `critical` | `--sev-critical: #ef4444` | animated `Flame` icon + uppercase localized text |

### Incident State

Implemented in `StateChip`.

| State          | Token                | Visual cue                                 |
| -------------- | -------------------- | ------------------------------------------ |
| `open`         | `--st-open: #ef4444` | `CircleDot` icon + colored rounded chip    |
| `acknowledged` | `--st-ack: #3b82f6`  | `Clock` icon + colored rounded chip        |
| `escalated`    | `--st-esc: #c084fc`  | `ShieldAlert` icon + colored rounded chip  |
| `resolved`     | `--st-res: #22c55e`  | `CheckCircle2` icon + colored rounded chip |

Release state visuals are not implemented in the current web client.

## 5. Reusable UI Classes

Implemented in `client-web/app/globals.css`.

| Class             | Purpose                                    |
| ----------------- | ------------------------------------------ |
| `.surface`        | bordered opaque panel                      |
| `.surface-subtle` | secondary panel background                 |
| `.glass`          | bordered panel with subtle inset highlight |
| `.ow-input`       | dark input with gold focus ring            |
| `.ow-primary`     | gold primary button                        |
| `.ow-secondary`   | dark bordered secondary button             |
| `.ow-danger`      | red destructive button                     |

Component-level patterns currently in use:

- `IncidentRow`: table row with state chip, title, short ID, severity, date, and action link.
- `StateChip`: state color + icon + text.
- `SeverityChip`: severity color + icon + text.
- `Timeline`: incident log entries and a compact command input.
- `AppShell`: responsive sidebar on desktop and bottom navigation on mobile.

## 6. Accessibility Status

Implemented:

- incident state and severity are not color-only; each uses color, icon, and text;
- most action buttons include visible text or an icon with nearby context;
- form controls in signup, onboarding, and incident creation generally use visible
  labels;
- keyboard focus styles are present on shared inputs and several primary actions.

Known gaps:

- `Timeline` currently uses a placeholder-only text input (`Type command or log
entry...`) and should receive a visible or screen-reader label;
- annotated screenshots have not been added yet;
- desktop accessibility cannot be evaluated until the desktop client exists.

## 7. Dark Patterns

Current rule: destructive actions should be explicit, visually distinct, and
placed where users expect them.

Implemented examples:

- logout and account deletion actions use the red `.ow-danger` style;
- account deletion uses a confirmation modal in `settings/page.tsx`.

Known gap:

- this document should be revisited when team deletion, member moderation,
  release cancellation, and Manager transfer are implemented, because those
  destructive workflows must name the affected resource in a confirmation dialog.

## 8. Screenshots

Annotated screenshots are not present yet. Add them only after the screens they
document are stable, so the file does not describe UI that no longer exists.
