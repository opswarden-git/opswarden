# OpsWarden UI/UX Guidelines

This document serves as the visual and interaction contract for the OpsWarden web and desktop clients, fulfilling the VIGIL requirements for visual consistency, accessibility, and user experience.

OpsWarden adopts a "Control Room / NOC" (Network Operations Center) aesthetic. The design is utilitarian, robust, and highly technical. It prioritizes function over form, drawing inspiration from tools like GitHub Actions, Jenkins, and Prometheus.

## 1. Typography

The interface uses two primary typefaces:

- **IBM Plex Mono** (Monospace): Used for all technical data, numbers, IDs, section headers, badges, and timestamps. It reinforces the technical, terminal-like aspect.
- **IBM Plex Sans** (Sans-Serif): Used for body text, descriptions, and user names, ensuring readability for long-form content.

### Hierarchy

- **Title (`h1`)**: 21px, font-weight 700.
- **Subtitle (`h2/h3`)**: 18px, font-weight 600.
- **Section Label**: 11px, IBM Plex Mono, uppercase, tracking (letter-spacing) 0.15em.
- **Body (`p`)**: 14px, IBM Plex Sans.
- **Microcopy (`small`)**: 12px, color muted.

## 2. Color Palette & Usage Rules

OpsWarden uses a strict dark theme. Colors are never used randomly; they carry specific semantic meaning.

### Base Colors (The Environment)

| Color                      | Hex       | Usage                                                                                                                    |
| -------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------ |
| **Background Base**        | `#15161a` | The main application background (deepest charcoal).                                                                      |
| **Panel / Surface**        | `#0e0e12` | Cards, sidebars, and elevated containers.                                                                                |
| **Panel Hover**            | `#1b1b20` | Hover states for clickable rows and secondary buttons.                                                                   |
| **Border**                 | `#26262b` | Dividers and default card borders.                                                                                       |
| **Text Primary**           | `#e7e7ea` | Standard text and headings.                                                                                              |
| **Text Muted**             | `#8e9197` | Subtitles, secondary information, empty states.                                                                          |
| **Brand / Primary Action** | `#f1cf13` | The "Gold" accent. Used for primary buttons, active tabs, and highlights. It contrasts heavily with the dark background. |

### Severity Colors (Incidents)

Severity indicates the _impact_ of an incident.

| Severity     | Color  | Hex       | Usage Context                               |
| ------------ | ------ | --------- | ------------------------------------------- |
| **Low**      | Blue   | `#3b82f6` | Minor issues, no customer impact.           |
| **Medium**   | Amber  | `#f59e0b` | Partial degradation.                        |
| **High**     | Orange | `#fb7d3c` | Major degradation, core features affected.  |
| **Critical** | Red    | `#ef4444` | Complete outage, immediate action required. |

### State Colors (Lifecycle)

State indicates the _status_ of an incident.

| State            | Color  | Hex       | Usage Context                                     |
| ---------------- | ------ | --------- | ------------------------------------------------- |
| **Open**         | Red    | `#ef4444` | New, unacknowledged incident. Requires attention. |
| **Acknowledged** | Blue   | `#3b82f6` | A responder is on it.                             |
| **Escalated**    | Purple | `#c084fc` | More help is needed, severity likely increased.   |
| **Resolved**     | Green  | `#22c55e` | The issue is fixed and the system is stable.      |

_Note: For Releases, `Blocked` uses Red, `In Progress` uses Gold, and `Completed` uses Green._

## 3. Accessibility (a11y)

The interface follows strict accessibility minimums:

1. **Keyboard Navigation**: All primary actions (Create Incident, Acknowledge, Resolve, Change Severity) are fully reachable via `Tab` and actionable via `Enter`/`Space`.
2. **Explicit Labels**: No input relies solely on placeholders. Every `<input>` or `<select>` is tied to a visible `<label>`.
3. **No Color-Only Information**: **"Color is never the only signal"**. Every state or severity indicator is always accompanied by an icon and/or explicit text.
   - _Example_: A "Critical" incident isn't just a red dot; it is a chip containing a specific alert icon `TriangleAlert` + the word "Critical" + the color red. This ensures full readability for color-blind users.

## 4. Reusable UI Components

To maintain consistency, the app relies on predefined components:

- **Button**:
  - `Primary (Gold)`: For the main action of a view (e.g., "Create Incident").
  - `Secondary (Panel)`: For standard actions (e.g., "Edit").
  - `Ghost`: For low-priority or icon-only actions.
- **Status Chip**: A pill-shaped indicator displaying a severity or state. Always uses a background with 12% opacity of the main color, and a border with 30% opacity.
- **Data Row**: Used for lists of incidents or releases. Features a fixed-width Mono ID (`#INC-102`), a truncated title, and right-aligned metadata/chips. Hovering highlights the row.
- **KPI Card**: Displays a single large metric (e.g., "MTTR") in Mono font, with an uppercase tracking label above it.

## 5. Dark Patterns & Mitigation

OpsWarden prohibits dark patterns. The user must always be in control.

- **Destructive Actions**: Deleting an incident, kicking a member, or transferring the Manager role are considered destructive.
  - _Mitigation_: These actions never trigger immediately. They always open a `ConfirmDialog` modal. The modal explicitly names the affected resource (e.g., _"Are you sure you want to kick **Alice** from the team?"_).
- **No Confirmation Inversion**: Buttons are explicitly named ("Confirm Delete" vs "Cancel"). We do not use trick phrasing like "Click here to not cancel".
- **Visibility**: Settings and destructive options are located in logical places (e.g., "Team Settings") and are never hidden behind non-obvious UI affordances.

## 6. Annotated Screenshots

_(To be added by the developer during S6 after final polish)_

1. **Dashboard & Navigation**: Showing the sidebar, KPIs, and the active incident list.
2. **Incident War Room**: Showing the timeline, presence indicators, and state/severity controls.
