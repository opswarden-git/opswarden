# UI guidelines

**Owner:** web client maintainers · **Status:** implemented contract · **Review:**
when shared components or tokens change

These guidelines turn the OpsWarden identity into repeatable interface
decisions. They apply to the Next.js client and to the Tauri shell that embeds
it. The implementation in `client-web/app/globals.css` and
`client-web/components/ui/` remains the source of truth.

## Brand foundations

The Pomelli boards below are supporting brand exploration. They are useful for
composition and tone, but they do not override implemented tokens, accessible
contrast or responsive behavior.

![OpsWarden brand overview](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/01-brand-overview.png){ .brand-board loading=lazy }

### Voice and promise

- Product name: **OpsWarden**.
- Preferred line: **Ship fearlessly, resolve instantly.**
- Voice: direct, calm and operational. State what happened, its impact and the
  next safe action; avoid hype, blame and unexplained acronyms.
- Labels describe outcomes: “Resolve incident”, “Validate step” and “Transfer
  Manager”, not vague verbs such as “Submit” or “Continue”.

### Logo

![OpsWarden logo exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/02-logo-usage.png){ .brand-board loading=lazy }

- Use the complete wordmark when the product identity is not already visible;
  use the shield icon for compact navigation, favicons and app icons.
- Keep clear space of at least **1× the inner eye diameter** on every side. The
  board's fixed `95 px` value is illustrative, not a responsive rule.
- Keep the full lockup at least `152 px` wide and the icon at least `24 px`.
- Never stretch, rotate, recolor, outline or add shadows to the mark.
- On dark surfaces use the supplied light wordmark; on light surfaces use its
  approved dark variant. Never place it over busy operational content.

### Typography

![OpsWarden typography exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/03-typography.png){ .brand-board loading=lazy }

| Family             | Purpose                                          | Rule                                             |
| ------------------ | ------------------------------------------------ | ------------------------------------------------ |
| **Inter**          | navigation, headings, body, controls             | Default interface family; sentence case          |
| **JetBrains Mono** | IDs, timestamps, routes, code and machine values | Use only where fixed-width scanning adds meaning |

Do not introduce a third family. A table may use Inter for human-readable names
and JetBrains Mono for an Incident identifier in the same row.

### Canonical palette

![OpsWarden color exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/04-color-palette.png){ .brand-board loading=lazy }

Pomelli's generated color names are not canonical. Use semantic roles instead:

| Token role              | Value                 | Use                               |
| ----------------------- | --------------------- | --------------------------------- |
| Control-room background | `#15161A`             | Application canvas                |
| Surface                 | `#1B1C20`             | Panels, fields, secondary actions |
| Primary text            | `#E7E7EA`             | High-priority readable content    |
| OpsWarden accent        | `#FBC02D`             | Primary action and focus          |
| Destructive action      | `#C62828`             | Explicit destructive controls     |
| Muted text              | `#989BA1` / `#878B93` | Supporting information only       |

Never use the accent as decoration around dense operational data. Color is
always paired with text, an icon or position when it communicates state.

## Operational semantics

Brand colors identify OpsWarden; operational colors encode live meaning. Every
operational badge uses the same opaque panel primitive and one of five
accessible emphasis tones:

| Tone    | Token              | Value     | White contrast | Meaning                      |
| ------- | ------------------ | --------- | -------------: | ---------------------------- |
| Neutral | `--status-neutral` | `#57606A` |         6.39:1 | Initial, inactive or ended   |
| Info    | `--status-info`    | `#0969DA` |         5.19:1 | Work is owned or in progress |
| Warning | `--status-warning` | `#9A6700` |         4.87:1 | Attention without failure    |
| Danger  | `--status-danger`  | `#CF222E` |         5.36:1 | Escalation, block or failure |
| Success | `--status-success` | `#1A7F37` |         5.08:1 | Successful terminal state    |

The lifecycle mapping is the **Progression** model: an open Incident starts
neutral, becomes informational once acknowledged, becomes danger when
escalated and success when resolved.

Every badge below carries its own name as text, verbatim: the tone never
travels alone. The severity silhouette is progressively stronger, and this is
the original inventory grammar, retained inside the opaque panels.

**Severity** — how bad it is.

| State    | Tone    | Icon           |
| -------- | ------- | -------------- |
| Low      | Neutral | circle alert   |
| Medium   | Warning | triangle alert |
| High     | Warning | octagon alert  |
| Critical | Danger  | flame          |

**Incident state** — where it is in its life.

| State        | Tone    |
| ------------ | ------- |
| Open         | Neutral |
| Acknowledged | Info    |
| Escalated    | Danger  |
| Resolved     | Success |

**Release state** — its own family, for its own object.

| State       | Tone    |
| ----------- | ------- |
| Created     | Neutral |
| In progress | Info    |
| Blocked     | Danger  |
| Completed   | Success |
| Cancelled   | Neutral |

Connections, rules and automation runs use the same five-tone vocabulary. Team
roles remain light identity labels with distinct shields. An active ban is an
enforced restriction and therefore uses the Danger panel with a Ban icon; an
expired ban is historical metadata and uses a neutral Clock icon without a
background. Presence remains an indicator and filters remain controls.

**Color is never the only signal.** Every state is carried by color **and** icon
**and** text. An icon is unique within its family, so two states of the same
object can never render the same glyph.

The server owns allowed lifecycle transitions. A colored chip renders state; it
does not authorize a transition or invent a new one.

### Visual status reference

These plates document the canonical mapping for the opaque status grammar. The
implemented shape is the prototype's **Panneau** option: a 4 px corner radius.
They document the five emphasis tones and every operational family without
adding binary assets to the application repository.

![Accessible emphasis palette](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/status-badges/01-palette-emphasis.png){ .annotated-screen loading=lazy }

![Incident status and severity](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/status-badges/02-incidents.png){ .annotated-screen loading=lazy }

![Release lifecycle](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/status-badges/03-delivery.png){ .annotated-screen loading=lazy }

![Team roles and access restrictions](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/status-badges/04-teams-access.png){ .annotated-screen loading=lazy }

![Connection, rule and automation run status](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/status-badges/05-automations-integrations.png){ .annotated-screen loading=lazy }

## Spacing and cadence

One shared cadence, Primer's base scale, in pixels:

`2 · 4 · 6 · 8 · 12 · 16 · 20 · 24 · 28 · 32 · 36 · 40 · 44 · 48 · 64 · 80 · 96 · 112 · 128`

10 and 14 are skipped on purpose. The damage they did was not that either value
looked wrong on its own: the button ramp read 6 · 12 · 14 · 16, so `md` and `lg`
sat two pixels apart while `sm` and `md` sat twelve. A shared cadence is what
makes two screens performing similar actions look alike.

`56px` is the single allowed exception — a page-level value whose neighbours are
far enough away that the step is not perceptible.

Within that cadence, compose from a short vocabulary rather than the whole
ramp. Primer exposes six functional steps — `2 · 4 · 8 · 12 · 16 · 24` — and
reserves the rest of the scale for layout values that are chosen deliberately.
Nineteen legal steps is not a cadence, it is a permission: two panels doing the
same job end up 20px and 24px apart for no reason a reader can recover.

Scope is spacing: padding, margin and gap. Sizing is excluded, because
`h-3.5 w-3.5` is the 14px inline icon size used across the product and that is a
deliberate choice, not a spacing decision. Arbitrary values such as `p-[13px]`
are rejected outright.

## Surfaces

Every surface that holds content — a card, a dialog, a sheet, a side panel —
uses the same three parts, and only the first and last are optional:

`header?` · `body` · `footer?`

A card is a dialog that does not float. Keeping one grammar means a decision
about where a title sits, or where a line belongs, is taken once instead of
once per surface. `Dialog` marks these parts with `data-dialog-part`, so a test
can assert against them without a screenshot.

Three rules follow from the grammar:

- **A surface does not contain a bordered surface.** Depth comes from the
  background plane and from spacing, not from stacking outlines. When a form
  needs groups, separate them with space and a label, not with a boxed
  `fieldset` inside a boxed dialog.
- **A horizontal line answers an overflow.** Use `.scroll-divider`, which shows
  its rule only while content can scroll under it. Do not draw a line to
  decorate the seam between two parts that both fit on screen.
- **A record is inset the same everywhere.** Sixteen pixels on the horizontal,
  in a table cell and in a list row alike, so a record starts at the same place
  whichever surface renders it. Only the vertical varies, and only with density:
  8px in a compact table, 12px in a normal one, 16px in a list row. Reach for
  the shared component before choosing a number — seven different insets once
  coexisted here, and every one of them was individually defensible.
- **The subtitle is not the title again.** A subtitle carries what the title
  cannot: the named resource, the consequence, the count. If it restates the
  title in a longer form, delete it and use `titleHidden` when the trigger
  already named the action.

The narrow-viewport presentation of a dialog is a position, not a different
component: the same surface, the same parts, anchored to the bottom edge.

## Components

Use a shared primitive before adding local CSS. A new variant requires a product
meaning, not merely a different color.

| Component          | Variants or states                        | Usage contract                                                  |
| ------------------ | ----------------------------------------- | --------------------------------------------------------------- |
| `Button`           | `primary`, `secondary`, `danger`, `ghost` | One dominant primary action per decision area                   |
| `IconButton`       | same intent system                        | Must have an accessible name and tooltip where helpful          |
| `Alert`            | `info`, `success`, `warning`, `danger`    | Message tone; never styled like an action                       |
| `FormField`        | label, hint, error                        | Visible label; connect description and error programmatically   |
| `Dialog`           | open/closed                               | Trap focus, close with Escape, restore focus to trigger         |
| `ConfirmDialog`    | standard/destructive                      | Initial focus on the safe action; name resource and consequence |
| `ActionMenu`       | closed/open                               | Keyboard-openable; arrows navigate; Escape returns focus        |
| `OperationalTable` | loading/ready                             | Desktop scan view with a labelled mobile record equivalent      |
| `PageContent`      | loading/error/empty/ready                 | Every data page handles all four states                         |
| `StatusBadge`      | neutral/info/warning/danger/success       | Opaque 4 px panel; icon and translated text are mandatory       |

### Action hierarchy

1. Primary: the single constructive outcome of the current task.
2. Secondary: cancel, close, return or a valid alternative.
3. Ghost: low-emphasis contextual action.
4. Danger: destructive outcome, separated from routine actions.

Do not disable a control without explaining why nearby. During a request, keep
the label stable when possible and expose busy state to assistive technology.

## Layout and responsive behavior

- Use `AppShell`, `PageLayout`, `PageHeader`, `PageToolbar` and `PageContent` to
  preserve navigation and loading behavior.
- Put the operational identity and status before metadata and actions.
- Prefer a table for repeated desktop records and a labelled record stack on
  small screens. Preserve the same information and action names in both.
- Keep dialogs for focused decisions; do not turn long exploration workflows
  into nested dialogs.
- Dense data may scroll horizontally only when a meaningful mobile equivalent
  cannot preserve it.

### A room has a fixed frame

An incident is a place, not a record, and its screen is built accordingly.

- The heading stays put, the composer stays pinned, and **only the transcript
  scrolls**. You can read back through an incident without losing the way to
  answer it.
- The composer sits **below** the transcript. At the top it reads "post an
  update"; at the bottom, after what has already been said, it reads as
  answering — the difference between a feed and a room.
- Consecutive notes from one author collapse into a single block — one avatar,
  one name, one timestamp — within a **two-minute** window. Mattermost uses five;
  during a response messages arrive in bursts, and five minutes would swallow
  distinct turns of a conversation.
- **System events never join a block.** A status change, an assignment or an
  escalation is precisely what someone re-reading an incident is looking for, and
  it must never be absorbed into a series of notes.
- A system event is never deduplicated or collapsed into `×N`: one persisted
  event produces one transcript row. Lifecycle and severity transitions render
  their canonical before/after badges inline so the change scans instantly.
- Below `lg` the context becomes an on-demand sheet rather than a stacked panel:
  with a fixed frame, a panel under a scrolling transcript would sit behind the
  entire conversation.

## Accessibility contract

Every production flow must satisfy all of these rules:

- A native form control has an explicit accessible name; a placeholder is not a label.
- Keyboard users can open, navigate and close dialogs and menus, with visible focus.
- Focus starts predictably and returns to the invoking control.
- Validation and request failures are announced live and remain readable.
- No positive `tabIndex` creates a manual tab order.
- Meaning never depends on color alone.
- English and French messages preserve the same ICU arguments and interaction meaning.
- Motion respects reduced-motion preferences and never delays an urgent action.

These are enforced by component tests, flow tests and the static accessibility
contract. A visual review supplements those checks; it does not replace them.

## Destructive flows and dark patterns

Persistent deletion, expulsion, banning and irreversible cancellation use a
`ConfirmDialog` with explicit intent. Name the affected resource, explain the
consequence, focus the safe action first and never preselect danger. High-impact
deletion may require typing `DELETE`.

Do not add confirmation friction to immediately reversible changes such as a
filter, emoji reaction or rule toggle. Never hide cancellation, use guilt copy,
mislabel consequences or give the risky action stronger visual weight before
the user chooses it.

## Annotated product examples

### Incident queue: scan before action

![Incident queue showing operational hierarchy](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/incidents.png){ .annotated-screen loading=lazy }

1. The page title and team context establish scope before any mutation.
2. Severity and lifecycle are both written and color-coded for fast scanning.
3. The primary incident action is visually distinct from filters and row actions.
4. Repeated records keep identifiers, owners and timestamps aligned; the mobile
   representation must retain those facts.

### Release queue: make blocking explicit

![Release queue showing progress and blocking state](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/readme/releases.png){ .annotated-screen loading=lazy }

1. Release state and ordered progress answer “can we continue?” first.
2. A blocked release names its condition rather than relying on danger color.
3. Step validation remains a deliberate action and follows server-owned order.
4. Cancel is destructive and separated from the normal validation path.

## What a machine checks

Most of this document is a contract, not advice. These rules fail the build
rather than a review:

| Rule                                                   | Contract                                 |
| ------------------------------------------------------ | ---------------------------------------- |
| Token families exist and clear 4.5:1 contrast          | `app/design-tokens.test.ts`              |
| A release state never wears an incident token          | `app/design-tokens.test.ts`              |
| Every state renders color **and** icon **and** text    | `components/state-encoding.test.ts`      |
| Status panels stay opaque, compact and accessible      | `components/ui/StatusBadge.test.tsx`     |
| Spacing stays on the shared cadence                    | `components/spacing-scale.test.ts`       |
| Radius and border weights keep their documented values | `app/design-tokens.test.ts`              |
| Dialog dividers appear only over scrollable content    | `app/design-tokens.test.ts`              |
| Interface prose stays within its per-locale budget     | `i18n/text-budget.test.ts`               |
| Destructive flows name resource and consequence        | `components/destructive-actions.test.ts` |

They read the source rather than a rendered screen, so they hold whatever a
component looks like. A visual review supplements them; it does not replace
them.

## Review checklist

Before merging an interface change, verify:

- [ ] Existing tokens and shared components are reused.
- [ ] Loading, error, empty and ready states are present.
- [ ] Desktop and narrow viewport flows preserve information and actions.
- [ ] Keyboard focus, names and live errors work without a mouse.
- [ ] EN and FR copy is complete and natural, and fits the text budget — or the
      ceiling is raised deliberately, with the reason recorded.
- [ ] No bordered surface sits inside another bordered surface.
- [ ] Every horizontal line separates something that can actually scroll.
- [ ] Destructive consequences are named; reversible actions stay lightweight.
- [ ] Component, integration or E2E coverage protects the new behavior.

See also the lower-level
[visual contract](https://opswarden-git.github.io/opswarden/design/design-system/)
and the component implementations under `client-web/components/ui/`.
