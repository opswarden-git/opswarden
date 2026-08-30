# Design system

This document is the technical token and primitive reference shared by the web
and desktop clients. Canonical values live in `client-web/app/globals.css`;
shared components consume semantic roles instead of choosing values screen by
screen. Product usage, accessibility and annotated examples belong in the
[`UI_GUIDELINES.md` portal page](https://opswarden-git.github.io/opswarden/design/ui-guidelines/).

## Primary palette — 5 colors

| Color     | Primary role                               |
| --------- | ------------------------------------------ |
| `#15161A` | Control-room background                    |
| `#1C1D22` | Surfaces, fields and secondary actions     |
| `#E7E7EA` | Primary text and high-priority content     |
| `#FBC02D` | OpsWarden accent, focus and primary action |
| `#C62828` | Destructive actions and danger             |

Hover variants are derived from these same roles. Operational state and severity
colors are not additional brand colors: they only encode live meaning and are
always paired with text or an icon.

## Semantic roles

| Role               | Tokens                | Usage                                             |
| ------------------ | --------------------- | ------------------------------------------------- |
| Primary action     | `--action-primary*`   | Main constructive action in a page or dialog      |
| Secondary action   | `--action-secondary*` | Cancel, close, go back or choose an alternative   |
| Destructive action | `--action-danger*`    | Delete, remove, ban, leave or irreversibly cancel |
| Success            | `--feedback-success`  | Operation completed successfully                  |
| Warning            | `--feedback-warning`  | Risk or required attention without a failure      |
| Danger             | `--feedback-danger`   | Error, blocked operation or critical state        |

`Button` has exactly four variants: `primary`, `secondary`, `danger` and
`ghost`. `Alert` has `info`, `success`, `warning` and `danger`. An action must
never use a message color, and a message must never look like a button.

## Domain state families

Every domain state is conveyed by color **and** icon **and** text — color is
never the only signal. Domain components map their state onto one shared,
opaque `StatusBadge` panel vocabulary:

| Tone    | Token              | Value     | Use                                       |
| ------- | ------------------ | --------- | ----------------------------------------- |
| Neutral | `--status-neutral` | `#57606A` | Initial, inactive, skipped or cancelled   |
| Info    | `--status-info`    | `#0969DA` | Acknowledged, running or in progress      |
| Warning | `--status-warning` | `#9A6700` | Medium/high severity or awaiting an event |
| Danger  | `--status-danger`  | `#CF222E` | Escalated, critical, blocked or failed    |
| Success | `--status-success` | `#1A7F37` | Resolved, completed or verified           |

The selected Incident model is **Progression**: Open is neutral, Acknowledged
is info, Escalated is danger and Resolved is success. The shape is always a
compact **panel** with a 4 px radius. Metadata labels, counters, presence and
filters keep their own lighter grammar. Team roles and access restrictions
remain identity metadata: roles use a distinct shield plus neutral text, while
an active ban uses the Danger status panel and an expired ban falls back to
neutral clock metadata.

## Shape and borders

Radius identifies the kind of box, so the four implemented values are not
interchangeable.

| Token              | Value  | Use for                              | Never for          |
| ------------------ | ------ | ------------------------------------ | ------------------ |
| `--ow-radius-sm`   | 3px    | Badges, tags, inline marks           | Cards or buttons   |
| `--ow-radius-md`   | 6px    | Default: buttons, inputs, containers | —                  |
| `--ow-radius-lg`   | 12px   | Dialogs and sheets                   | Row-level elements |
| `--ow-radius-full` | 9999px | Avatars, pills, presence dots        | Rectangular boxes  |

Tailwind radius utilities read these tokens, so utilities and CSS variables
cannot drift apart.

Conversation bubbles are the single documented exception: they keep a larger
radius with one squared corner, because that asymmetry is what makes a bubble
read as speech rather than as a card.

Three neutral border weights distinguish an internal separator, a surface edge
and an interactive control.

| Token                  | Alpha | Meaning                                    |
| ---------------------- | ----- | ------------------------------------------ |
| `--ow-border-muted`    | 5%    | Separator between rows inside a surface    |
| `--ow-border`          | 8%    | Edge of a surface                          |
| `--ow-border-emphasis` | 12%   | Control the user can act on, such as input |

`--ow-border-width` is 1px. `--ow-border-width-thick` is 2px and is reserved for
focus rings.

A divider answers an overflow; it is not decoration. `.scroll-divider` draws
its rule only while content can scroll under it. Browsers without scroll-driven
animations keep the rule as a safe fallback.

## Hierarchy and surfaces

- One primary action leads each decision area.
- Secondary actions appear before destructive actions in dialogs.
- `surface` holds primary content; `surface-subtle` groups supporting details;
  `surface-floating` is the same surface once it leaves the plane, and only
  changes its radius.
- Screens use the shared `Button`, `IconButton`, `Alert`, `Dialog`,
  `ConfirmDialog`, `FormField`, `ActionMenu` and `OperationalTable` components.
- `PageContent` owns the loading, error, empty and ready states.
- Responsive layouts preserve labels and action hierarchy; only the table/list
  or modal/sheet presentation changes.

## Primitive invariants

- `Button` owns action intent; `Alert` owns feedback tone.
- `StatusBadge` is opaque and always combines icon, text and semantic tone.
- `ConfirmDialog` requires explicit intent, starts focus on the safe action and
  never preselects destruction.
- `Dialog` and `ActionMenu` restore focus to their trigger.
- `PageContent` owns loading, error, empty and ready states.

These invariants are verified by `design-tokens.test.ts`,
`state-encoding.test.ts`, `destructive-actions.test.ts`, `Button.test.tsx`,
`Alert.test.tsx` and `ConfirmDialog.test.tsx`. The complete sensitive-action and
dark-pattern policy is defined once in `UI_GUIDELINES.md`.
