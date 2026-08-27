import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

/**
 * A ratchet on how much prose the interface carries.
 *
 * The audit measured the Team overview at 73% explanatory prose — 89 words of
 * `*Description` and empty states against 32 words of actual labels. Cleaning
 * one screen achieves nothing if the next one re-grows: nothing in the test
 * suite noticed the weight in the first place.
 *
 * These ceilings are the measured state on 2026-08-04, not a target. They can
 * only go down. Raising one is allowed but must be deliberate: a failing
 * assertion naming the namespace is the point, not an obstacle.
 *
 * Both locales are checked. French is systematically longer than English, so a
 * budget held only on `en` would let the interface a French-speaking jury sees
 * drift unmeasured.
 */

const LOCALES = ["en", "fr"] as const;

/** Words a reader actually reads: ICU placeholders carry no prose. */
function words(value: unknown): number {
  if (typeof value === "string") {
    return (value.replace(/\{[^}]*\}/g, " ").match(/[A-Za-z0-9'’]+/g) ?? []).length;
  }
  if (value && typeof value === "object") {
    return Object.values(value).reduce<number>((total, entry) => total + words(entry), 0);
  }
  return 0;
}

function messages(locale: string): Record<string, unknown> {
  return JSON.parse(
    fs.readFileSync(path.join(process.cwd(), "messages", `${locale}.json`), "utf8"),
  );
}

/**
 * Measured 2026-08-04 and deliberately re-measured 2026-08-24 after restoring
 * actionable messaging errors and giving each conversation surface ownership
 * of its visible labels. Per locale rather than through a multiplier: French runs
 * 30% heavier than English overall, and unevenly — 8% on DirectMessages, 65% on
 * Notifications. A single coefficient would leave slack on some namespaces and
 * fail honest translations on others.
 *
 * `Teams` moves by two words on 2026-08-27: the invitation code gained the
 * label it never had, so the value a manager is asked to share is now named
 * like every other field in the product instead of floating unlabelled.
 *
 * Four ceilings come down on 2026-08-27, to the weight left after deleting the
 * dialog subtitles that only restated their title. A subtitle earns its place by
 * carrying what the title cannot — the named resource, the consequence, the
 * count — and six of them carried nothing: "Filters / Filter and sort this
 * table." The two that survived say what happens next, not what the button
 * already said.
 *
 * `Sidebar` carries the first-run guidance from 2026-08-27: four sentences, one
 * per empty section, plus the marker's accessible name and the acknowledgement.
 * It is the largest single rise in this table and it is deliberate — this prose
 * exists only in a workspace with nothing in it, and disappears for good on the
 * first incident, release, connection or rule. A reader who has used the
 * product once never meets it again.
 *
 * The earlier note on this namespace:
 * `Sidebar` gained one label on 2026-08-27: the first-run marker needs a name a
 * screen reader can read, since a coloured dot says nothing on its own. Three
 * words for the one affordance that tells a brand-new workspace where to start.
 *
 * `Settings` rises on purpose on 2026-08-27, from 100/127, for the desktop
 * notification permission control. Browsers only honour a permission request
 * that follows a click, so the interface has to own that click and then say
 * which of four answers it got back — on, off, blocked in the browser, or
 * unavailable here. The blocked state is the one the app cannot recover from
 * alone, so it names where to undo it rather than leaving a dead toggle.
 * The same section adds six words per locale for the explicit,
 * persistent notification-sound opt-in. Its on/off values reuse existing copy.
 *
 * `Notifications` rises on purpose on 2026-08-27, from 23/38 to the measured
 * weight of the desktop notification vocabulary. The namespace grew from a
 * handful of strings to sixteen, covering every event the OS is allowed to
 * surface: incident assigned, critical, escalated, state changed, War Room
 * message, direct message, release step validated, release state changed and
 * release blocked. Each one is a title and a body of an OS notification, where
 * the text is the entire interface — there is no surrounding screen to carry
 * meaning. The prose is already at its floor: `Release #{id} is now {state}`
 * cannot be shortened without losing which release or which state. This is a
 * larger vocabulary, not looser writing.
 *
 * `Common` rises once here, on purpose: the nine GIPHY picker strings moved out
 * of `Incidents` into the shared namespace they are actually rendered from, and
 * the two that had been copied into `DirectMessages` collapsed back into one
 * entry. The interface carries three fewer English words and four fewer French
 * ones than before the move — a raise on one namespace, not on the budget.
 * `Teams` adds the explicit self label used in the actionable presence list.
 * It rises by four English and six French words on 2026-08-27 for the two
 * accessible Team-image actions. These labels make an otherwise icon-only
 * upload and removal control understandable to assistive technology.
 */
const CEILINGS: Record<string, Record<string, number>> = {
  en: {
    Teams: 314,
    Incidents: 277,
    errors: 335,
    Automations: 280,
    Releases: 180,
    Onboarding: 90,
    Settings: 120,
    Auth: 31,
    DirectMessages: 56,
    Notifications: 45,
    Sidebar: 68,
    TeamSwitcher: 11,
    Metadata: 7,
    Common: 29,
    Index: 3,
  },
  fr: {
    Teams: 381,
    Incidents: 380,
    errors: 417,
    Automations: 398,
    Releases: 222,
    Onboarding: 110,
    Settings: 151,
    Auth: 39,
    DirectMessages: 71,
    Notifications: 71,
    Sidebar: 84,
    TeamSwitcher: 16,
    Metadata: 11,
    Common: 43,
    Index: 3,
  },
};

const total = (locale: string) =>
  Object.values(CEILINGS[locale]).reduce((sum, value) => sum + value, 0);

describe("interface text budget", () => {
  it("declares a budget for every namespace that ships", () => {
    for (const locale of LOCALES) {
      expect(Object.keys(messages(locale)).sort(), `${locale} namespaces`).toEqual(
        Object.keys(CEILINGS[locale]).sort(),
      );
    }
  });

  it("keeps every namespace within its measured weight", () => {
    for (const locale of LOCALES) {
      const bundle = messages(locale);
      for (const [namespace, budget] of Object.entries(CEILINGS[locale])) {
        const actual = words(bundle[namespace]);
        expect(
          actual,
          `${locale}.${namespace}: ${actual} words for a ${budget} budget — trim the prose or raise the ceiling on purpose`,
        ).toBeLessThanOrEqual(budget);
      }
    }
  });

  it("keeps the whole interface within its measured weight", () => {
    for (const locale of LOCALES) {
      const budget = total(locale);
      const actual = words(messages(locale));
      expect(
        actual,
        `${locale}: ${actual} words across the interface, budget ${budget}`,
      ).toBeLessThanOrEqual(budget);
    }
  });
});
