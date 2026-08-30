# UI guidelines

**Scope:** Next.js web client and shared Tauri interface · **Target:** WCAG 2.2
AA · **Source of truth:** `client-web/app/globals.css` and
`client-web/components/ui/`

OpsWarden is a dark operational interface: calm, dense and immediately
scannable. Every screen must reveal the current state, the affected resource
and the next safe action without relying on color alone. This two-page contract
summarizes the rules required for a consistent implementation; token-level
details live in `DESIGN_SYSTEM.md`.

## Foundations

### Palette — five principal colors

| Role      | Color     | Usage                                         |
| --------- | --------- | --------------------------------------------- |
| Primary   | `#FBC02D` | Constructive primary action and visible focus |
| Secondary | `#1C1D22` | Secondary controls and surfaces               |
| Success   | `#22C55E` | Successful feedback                           |
| Warning   | `#F59E0B` | Attention without failure                     |
| Danger    | `#C62828` | Destructive action                            |

The canvas (`#15161A`) and primary text (`#E7E7EA`) support these roles.
Operational badges use the darker accessible tones listed in the
[`DESIGN_SYSTEM.md` portal page](https://opswarden-git.github.io/opswarden/design/design-system/).
Accent color is not decoration. Any color that communicates meaning is paired
with explicit text and an icon.

### Typography and spacing

- **Inter** is used for navigation, titles, body text, labels and controls.
- **JetBrains Mono** is reserved for IDs, timestamps, routes and machine values.
- Title, subtitle and body form the minimum hierarchy; use sentence case and
  describe outcomes such as “Resolve incident”, never “Submit”.
- Spacing uses the shared grid `2 · 4 · 8 · 12 · 16 · 24` px. Larger layout
  values come from the implemented token scale; arbitrary spacing is forbidden.
- A record starts at the same 16 px horizontal inset in tables and lists.

## State grammar

Every status uses an opaque `StatusBadge` with a translated label and a unique
icon. The server owns transitions; the client only renders them.

| Object             | State → visual treatment                                                                                  |
| ------------------ | --------------------------------------------------------------------------------------------------------- |
| Incident severity  | Low → neutral circle · Medium → warning triangle · High → warning octagon · Critical → danger flame       |
| Incident lifecycle | Open → neutral · Acknowledged → info · Escalated → danger · Resolved → success                            |
| Release lifecycle  | Created → neutral · In progress → info · Blocked → danger · Completed → success · Cancelled → neutral     |
| Automation         | Inactive/skipped → neutral · running → info · attention → warning · failed → danger · succeeded → success |

The combination of object name, label, icon and stable placement makes severity
and status recognizable at a glance. Presence is an indicator, Team roles are
identity labels, and filters remain controls: they must not imitate operational
status badges.

## Components and actions

Use a shared primitive before adding local CSS. A new variant must represent a
product meaning, not a decorative preference.

| Component          | Variants / contract                                                             |
| ------------------ | ------------------------------------------------------------------------------- |
| `Button`           | `primary`, `secondary`, `danger`, `ghost`; one primary action per decision area |
| `FormField`        | Visible label, optional hint, programmatically connected error                  |
| `Alert`            | `info`, `success`, `warning`, `danger`; message styling never looks actionable  |
| `StatusBadge`      | Opaque neutral/info/warning/danger/success panel with icon and text             |
| `Dialog`           | Accessible name, predictable focus, Escape close and focus restoration          |
| `ConfirmDialog`    | Names the resource and consequence; safe action receives initial focus          |
| `ActionMenu`       | Keyboard opening and navigation; Escape restores trigger focus                  |
| `OperationalTable` | Labelled desktop table and equivalent mobile record projection                  |
| `PageContent`      | Explicit loading, error, empty and ready states                                 |

Action hierarchy is stable: **primary** for the constructive outcome,
**secondary** for cancel/close/alternative, **ghost** for low-emphasis context,
and **danger** for destruction. Critical actions are visible and never hidden
behind an unexplained icon or ambiguous wording.

Deleting a Team or Incident, kicking or banning a member, cancelling a Release
and transferring the Manager role require a dedicated confirmation dialog.
The dialog names the target and consequence, never reverses the question and
never preselects danger. Reversible actions such as filters, reactions and rule
toggles remain frictionless.

## Layout and responsive behavior

- Compose pages with `AppShell`, `PageLayout`, `PageHeader`, `PageToolbar` and
  `PageContent`; operational identity and state precede metadata and actions.
- Use tables for repeated desktop records and labelled record stacks on narrow
  screens, preserving the same information and action names.
- A content surface follows `header? · body · footer?` and does not contain
  another bordered surface. Dividers appear only when content can scroll.
- In an Incident room, the heading and composer stay fixed while only the
  transcript scrolls. System events remain individual visible rows.
- Below `lg`, secondary Incident context becomes an on-demand sheet rather than
  reducing the transcript to a narrow column.

## Accessibility and verification

OpsWarden targets **WCAG 2.2 Level AA** for both the web application and its
Tauri-rendered content. This is an engineering target, not a certification.

- Every native control has an explicit accessible name; placeholders are not
  labels. Primary Incident creation, acknowledgment, escalation and Release-step
  validation are keyboard accessible.
- Dialogs and menus expose visible focus, logical keyboard order and focus
  restoration. Validation and request errors remain readable and are announced.
- Text and status foregrounds meet at least 4.5:1 contrast. Meaning never depends
  on color alone, and reduced-motion preferences preserve state and feedback.
- French and English expose the same controls, state names and ICU arguments.

Compliance is checked by `app/design-tokens.test.ts` (tokens and contrast),
`components/accessibility-contract.test.ts` (names and native semantics),
`components/state-encoding.test.ts` (color + icon + text),
`components/spacing-scale.test.ts` (grid),
`components/destructive-actions.test.ts` (confirmations), component tests
(roles, live regions and focus), and Playwright critical paths at desktop and
narrow widths. Any failed automated contract blocks CI; keyboard-only, visible
focus and reduced-motion behavior also receive a manual release review.

## Annotated examples

### Incident queue

![Incident queue showing operational hierarchy](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/incidents.png){ .annotated-screen loading=lazy }

1. Team and page title establish scope before mutation.
2. Severity and lifecycle combine text, icon and color for instant scanning.
3. The primary creation action is distinct from filters and row actions.
4. IDs, owners and timestamps retain the same labels in the mobile projection.

### Release queue

![Release queue showing progress and blocking state](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/releases.png){ .annotated-screen loading=lazy }

1. State and ordered progress answer whether deployment can continue.
2. A blocked Release names its blocking condition instead of relying on red.
3. Step validation follows server-owned order and remains keyboard accessible.
4. Cancellation is separated from routine validation and requires confirmation.

## Dark patterns forbidden

Never hide cancellation, use guilt-inducing copy, disguise a critical action,
create a positive `tabIndex`, depend on placeholders, invert confirmation
wording or give a destructive option stronger emphasis before the user chooses
it. Before merging, verify desktop/mobile parity, EN/FR completeness, all four
data states, keyboard operation, destructive consequences and appropriate test
coverage.
