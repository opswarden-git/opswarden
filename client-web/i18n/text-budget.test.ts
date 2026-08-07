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
 * Measured 2026-08-04, per locale rather than through a multiplier: French runs
 * 30% heavier than English overall, and unevenly — 8% on DirectMessages, 65% on
 * Notifications. A single coefficient would leave slack on some namespaces and
 * fail honest translations on others.
 */
const CEILINGS: Record<string, Record<string, number>> = {
  en: {
    Teams: 451,
    Incidents: 343,
    errors: 331,
    Automations: 324,
    Releases: 248,
    Activity: 29,
    Onboarding: 166,
    Settings: 158,
    Auth: 31,
    DirectMessages: 26,
    Notifications: 23,
    // Activity is a new Manager-only destination, not duplicate page copy.
    Sidebar: 24,
    TeamSwitcher: 12,
    Metadata: 7,
    Common: 3,
    Index: 3,
  },
  fr: {
    Teams: 566,
    Incidents: 447,
    errors: 411,
    Automations: 442,
    Releases: 314,
    Activity: 38,
    Onboarding: 244,
    Settings: 198,
    Auth: 39,
    DirectMessages: 28,
    Notifications: 38,
    Sidebar: 32,
    TeamSwitcher: 17,
    Metadata: 11,
    Common: 5,
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
