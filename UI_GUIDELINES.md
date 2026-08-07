# OpsWarden UI guidelines

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

![OpsWarden logo exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/02-logo-usage.png){ .brand-board loading=lazy }

### Logo

- Use the complete wordmark when the product identity is not already visible;
  use the shield icon for compact navigation, favicons and app icons.
- Keep clear space of at least **1× the inner eye diameter** on every side. The
  board's fixed `95 px` value is illustrative, not a responsive rule.
- Keep the full lockup at least `152 px` wide and the icon at least `24 px`.
- Never stretch, rotate, recolor, outline or add shadows to the mark.
- On dark surfaces use the supplied light wordmark; on light surfaces use its
  approved dark variant. Never place it over busy operational content.

![OpsWarden typography exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/03-typography.png){ .brand-board loading=lazy }

### Typography

| Family             | Purpose                                          | Rule                                             |
| ------------------ | ------------------------------------------------ | ------------------------------------------------ |
| **Inter**          | navigation, headings, body, controls             | Default interface family; sentence case          |
| **JetBrains Mono** | IDs, timestamps, routes, code and machine values | Use only where fixed-width scanning adds meaning |

Do not introduce a third family. A table may use Inter for human-readable names
and JetBrains Mono for an Incident identifier in the same row.

![OpsWarden color exploration](https://raw.githubusercontent.com/wiki/opswarden-git/opswarden/assets/ui-guidelines/pomelli/04-color-palette.png){ .brand-board loading=lazy }

### Canonical palette

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

Brand colors identify OpsWarden; operational colors encode live meaning. There
are three families, and one never borrows another's vocabulary.

**Severity** — how bad it is.

| Concept  | Token            | Value     | Required text |
| -------- | ---------------- | --------- | ------------- |
| Low      | `--sev-low`      | `#5798F5` | Low           |
| Medium   | `--sev-medium`   | `#F59E0B` | Medium        |
| High     | `--sev-high`     | `#FB7D3C` | High          |
| Critical | `--sev-critical` | `#FF5555` | Critical      |

**Incident state** — where it is in its life.

| Concept      | Token       | Value     | Required text |
| ------------ | ----------- | --------- | ------------- |
| Open         | `--st-open` | `#FF5555` | Open          |
| Acknowledged | `--st-ack`  | `#5798F5` | Acknowledged  |
| Escalated    | `--st-esc`  | `#C084FC` | Escalated     |
| Resolved     | `--st-res`  | `#22C55E` | Resolved      |

**Release state** — its own family, for its own object.

| Concept     | Token             | Value     | Required text |
| ----------- | ----------------- | --------- | ------------- |
| Created     | `--rel-created`   | `#989BA1` | Created       |
| In progress | `--rel-progress`  | `#2DD4BF` | In progress   |
| Blocked     | `--rel-blocked`   | `#FF5555` | Blocked       |
| Completed   | `--rel-completed` | `#22C55E` | Completed     |
| Cancelled   | `--rel-cancelled` | `#878B93` | Cancelled     |

The release family exists because the overview shows incidents and releases side
by side. A release in progress used to wear `--st-ack`, the _incident
acknowledged_ token, so a single blue meant two different things depending on
which column was being read.

Where a release state expresses a concern the whole product shares, its token
aliases the `--feedback-*` role rather than an incident token: `--rel-blocked`
resolves to `--feedback-danger`, `--rel-completed` to `--feedback-success`.
Lifecycle roles may map onto concern colors; they never adopt another object's
vocabulary.

Severity and incident state do share two values — `#FF5555` and `#5798F5` — and
both chips can appear on the same row. They stay unambiguous because the rule
below is absolute, never because the hue is unique.

**Color is never the only signal.** Every state is carried by color **and** icon
**and** text. An icon is unique within its family, so two states of the same
object can never render the same glyph.

The server owns allowed lifecycle transitions. A colored chip renders state; it
does not authorize a transition or invent a new one.

## Spacing and cadence

One shared cadence, Primer's base scale, in pixels:

`2 · 4 · 6 · 8 · 12 · 16 · 20 · 24 · 28 · 32 · 36 · 40 · 44 · 48 · 64 · 80 · 96 · 112 · 128`

10 and 14 are skipped on purpose. The damage they did was not that either value
looked wrong on its own: the button ramp read 6 · 12 · 14 · 16, so `md` and `lg`
sat two pixels apart while `sm` and `md` sat twelve. A shared cadence is what
makes two screens performing similar actions look alike.

`56px` is the single allowed exception — a page-level value whose neighbours are
far enough away that the step is not perceptible.

Scope is spacing: padding, margin and gap. Sizing is excluded, because
`h-3.5 w-3.5` is the 14px inline icon size used across the product and that is a
deliberate choice, not a spacing decision. Arbitrary values such as `p-[13px]`
are rejected outright.

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

| Rule                                                | Contract                                 |
| --------------------------------------------------- | ---------------------------------------- |
| Token families exist and clear 4.5:1 contrast       | `app/design-tokens.test.ts`              |
| A release state never wears an incident token       | `app/design-tokens.test.ts`              |
| Every state renders color **and** icon **and** text | `components/state-encoding.test.ts`      |
| Spacing stays on the shared cadence                 | `components/spacing-scale.test.ts`       |
| Interface prose stays within its per-locale budget  | `i18n/text-budget.test.ts`               |
| Destructive flows name resource and consequence     | `components/destructive-actions.test.ts` |

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
- [ ] Destructive consequences are named; reversible actions stay lightweight.
- [ ] Component, integration or E2E coverage protects the new behavior.

See also the lower-level
[visual contract](https://opswarden-git.github.io/opswarden/design/design-system/)
and the component implementations under `client-web/components/ui/`.
