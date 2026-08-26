# Design system

This document defines the visual rules shared by the web and desktop clients.
Canonical values live in `client-web/app/globals.css`; shared components consume
semantic roles instead of choosing colors screen by screen.

## Primary palette — 5 colors

| Color     | Primary role                               |
| --------- | ------------------------------------------ |
| `#15161A` | Control-room background                    |
| `#1B1C20` | Surfaces, fields and secondary actions     |
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

## Hierarchy and surfaces

- One primary action leads each decision area.
- Secondary actions appear before destructive actions in dialogs.
- `surface` holds primary content; `surface-subtle` groups supporting details.
- Screens use the shared `Button`, `IconButton`, `Alert`, `Dialog`,
  `ConfirmDialog`, `FormField`, `ActionMenu` and `OperationalTable` components.
- `PageContent` owns the loading, error, empty and ready states.
- Responsive layouts preserve labels and action hierarchy; only the table/list
  or modal/sheet presentation changes.

## Sensitive-action and dark-pattern audit

`ConfirmDialog` requires an explicit `intent`. It places initial focus on the
cancel action, uses distinct labels, closes with Escape and never preselects the
risky action.

| Persistent flow      | Protection                                                         |
| -------------------- | ------------------------------------------------------------------ |
| Delete an account    | Named resource, destructive intent and typed `DELETE`              |
| Delete a team        | Named team, stated consequences and typed `DELETE`                 |
| Leave a team         | Named team, stated loss of access and destructive confirmation     |
| Transfer Manager     | Named recipient, stated role change and standard confirmation      |
| Kick or ban a member | Named member, stated consequence and destructive confirmation      |
| Delete an Incident   | Named Incident, typed `DELETE` and destructive confirmation        |
| Cancel a Release     | Named Release, explicit “keep” choice and destructive confirmation |
| Delete a rule        | Named rule, stated future impact and preserved history             |
| Disconnect a service | Named service and a clear description of removed data              |

Immediately reversible operations—filters, status changes, reactions,
non-Manager role changes, rule activation and Incident/Release links—add no
artificial friction. Removing a step while creating a Release destroys no
persisted data. Signing out is a secondary action, not a deletion.

These safeguards are verified by `design-tokens.test.ts`,
`destructive-actions.test.ts`, `Button.test.tsx`, `Alert.test.tsx` and
`ConfirmDialog.test.tsx`.
