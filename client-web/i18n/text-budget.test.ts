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
 * `Common` rises once here, on purpose: the nine GIPHY picker strings moved out
 * of `Incidents` into the shared namespace they are actually rendered from, and
 * the two that had been copied into `DirectMessages` collapsed back into one
 * entry. The interface carries three fewer English words and four fewer French
 * ones than before the move — a raise on one namespace, not on the budget.
 * `Teams` adds the explicit self label used in the actionable presence list.
 */
const CEILINGS: Record<string, Record<string, number>> = {
  en: {
    Teams: 329,
    Incidents: 293,
    errors: 335,
    Automations: 294,
    Releases: 219,
    Onboarding: 48,
    Settings: 100,
    Auth: 31,
    DirectMessages: 56,
    Notifications: 23,
    Sidebar: 26,
    TeamSwitcher: 11,
    Metadata: 7,
    Common: 29,
    Index: 3,
  },
  fr: {
    Teams: 401,
    Incidents: 402,
    errors: 417,
    Automations: 417,
    Releases: 280,
    Onboarding: 66,
    Settings: 127,
    Auth: 39,
    DirectMessages: 71,
    Notifications: 38,
    Sidebar: 38,
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
