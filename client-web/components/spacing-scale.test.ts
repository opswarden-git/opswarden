import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

/**
 * Keeps interface spacing on one shared cadence.
 *
 * The scale is Primer's, in pixels: no odd values, and 10 and 14 are skipped on
 * purpose. Before this contract the codebase used both, and the damage was not
 * that the values looked wrong on their own — it was that the button ramp read
 * 6 · 12 · 14 · 16, so `md` and `lg` sat two pixels apart while `sm` and `md`
 * sat twelve. A shared cadence is what makes two screens performing similar
 * actions look alike, which VIGIL grades under visual consistency.
 *
 * Scope is spacing only — padding, margin, gap. Sizing utilities are excluded:
 * `h-3.5 w-3.5` is the 14px inline icon size used across the product, which is
 * a deliberate choice and not a spacing decision.
 */

/** Primer base scale, in px. `0` is spacing removal, not a step. */
const SCALE = new Set([
  0, 2, 4, 6, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 64, 80, 96, 112, 128,
]);

/**
 * 56px (`p-14`) is kept by decision: it is a page-level layout value where the
 * gap to its neighbours is not perceptible, and the three uses are not adjacent
 * to anything on the fine end of the scale.
 */
const ALLOWED_EXCEPTIONS = new Set([56]);

const SPACING_PREFIXES = [
  "p",
  "px",
  "py",
  "pt",
  "pb",
  "pl",
  "pr",
  "m",
  "mx",
  "my",
  "mt",
  "mb",
  "ml",
  "mr",
  "gap",
  "gap-x",
  "gap-y",
  "space-x",
  "space-y",
];

function sourceFiles(directory: string): string[] {
  if (!fs.existsSync(directory)) return [];
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(target);
    return entry.name.endsWith(".tsx") && !entry.name.includes(".test.") ? [target] : [];
  });
}

type Usage = { where: string; utility: string; pixels: number };

function spacingUsages(): Usage[] {
  const roots = [path.join(process.cwd(), "components"), path.join(process.cwd(), "app")];
  const usages: Usage[] = [];

  for (const file of roots.flatMap(sourceFiles)) {
    const relative = path.relative(process.cwd(), file);
    fs.readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        for (const prefix of SPACING_PREFIXES) {
          // Negative look-behind on `[\w-]` keeps `gap-x-2` from also matching
          // as `x-2`, and `-mt-2` from being read as `mt-2`.
          const pattern = new RegExp(`(?<![\\w-])${prefix}-(\\[[^\\]]+\\]|[\\d.]+)(?![\\w-])`, "g");
          for (const match of line.matchAll(pattern)) {
            const raw = match[1];
            usages.push({
              where: `${relative}:${index + 1}`,
              utility: `${prefix}-${raw}`,
              // Tailwind spacing unit is 4px; an arbitrary value has no step.
              pixels: raw.startsWith("[") ? Number.NaN : Number(raw) * 4,
            });
          }
        }
      });
  }
  return usages;
}

describe("spacing scale contract", () => {
  const usages = spacingUsages();

  it("finds spacing to check", () => {
    expect(usages.length).toBeGreaterThan(100);
  });

  it("never reaches for an arbitrary spacing value", () => {
    const arbitrary = usages.filter((usage) => Number.isNaN(usage.pixels));
    expect(
      arbitrary.map((usage) => `${usage.where} ${usage.utility}`),
      "arbitrary spacing escapes the shared cadence",
    ).toEqual([]);
  });

  it("keeps every spacing value on the shared cadence", () => {
    const offScale = usages.filter(
      (usage) =>
        !Number.isNaN(usage.pixels) &&
        !SCALE.has(usage.pixels) &&
        !ALLOWED_EXCEPTIONS.has(usage.pixels),
    );
    expect(
      offScale.map((usage) => `${usage.where} ${usage.utility} = ${usage.pixels}px`),
      "off-scale spacing — round to a neighbouring step or document the exception",
    ).toEqual([]);
  });

  it("keeps 10px and 14px out of the scale", () => {
    const halfSteps = usages.filter((usage) => usage.pixels === 10 || usage.pixels === 14);
    expect(
      halfSteps.map((usage) => `${usage.where} ${usage.utility}`),
      "10px and 14px were removed on purpose; they blur the ramp between neighbouring steps",
    ).toEqual([]);
  });
});
