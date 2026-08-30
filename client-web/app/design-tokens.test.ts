import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesheet = readFileSync(resolve(process.cwd(), "app/globals.css"), "utf8");
const buttonSource = readFileSync(resolve(process.cwd(), "components/ui/Button.tsx"), "utf8");
const alertSource = readFileSync(resolve(process.cwd(), "components/ui/Alert.tsx"), "utf8");
const designSystem = readFileSync(resolve(process.cwd(), "../docs/DESIGN_SYSTEM.md"), "utf8");
const releaseChipSource = readFileSync(
  resolve(process.cwd(), "components/releases/ReleaseStateChip.tsx"),
  "utf8",
);
const statusBadgeSource = readFileSync(
  resolve(process.cwd(), "components/ui/StatusBadge.tsx"),
  "utf8",
);

const p4RegressionPairs = [
  { legacyForeground: "--ow-muted-2", foreground: "--ow-muted-2", background: "--bg" },
  { legacyForeground: "--ow-muted-2", foreground: "--ow-muted-2", background: "--panel" },
  { legacyForeground: "--ow-muted-2", foreground: "--ow-muted-2", background: "--panel-2" },
  { legacyForeground: "--danger", foreground: "--danger-text", background: "--panel-2" },
  { legacyForeground: "--sev-low", foreground: "--sev-low", background: "--panel-2" },
  {
    legacyForeground: "--sev-critical",
    foreground: "--sev-critical",
    background: "--panel-2",
  },
  { legacyForeground: "--st-open", foreground: "--st-open", background: "--panel-2" },
  { legacyForeground: "--st-ack", foreground: "--st-ack", background: "--panel-2" },
] as const;

function cssToken(name: string): string {
  const value = stylesheet.match(new RegExp(`${name}:\\s*([^;]+);`, "i"))?.[1].trim();
  if (!value) throw new Error(`Missing token ${name}`);
  const reference = value.match(/^var\((--[a-z0-9-]+)\)$/i)?.[1];
  return reference ? cssToken(reference) : value;
}

function rgb(hex: string) {
  const value = hex.slice(1);
  const expanded = value.length === 3 ? [...value].map((digit) => digit.repeat(2)).join("") : value;
  return expanded
    .match(/.{2}/g)!
    .slice(0, 3)
    .map((channel) => Number.parseInt(channel, 16) / 255);
}

function luminance(hex: string) {
  const [red, green, blue] = rgb(hex).map((channel) =>
    channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(foreground: string, background: string) {
  const light = Math.max(luminance(foreground), luminance(background));
  const dark = Math.min(luminance(foreground), luminance(background));
  return (light + 0.05) / (dark + 0.05);
}

function composite(foreground: string, background: string, opacity: number) {
  const foregroundChannels = rgb(foreground);
  const backgroundChannels = rgb(background);
  const channels = foregroundChannels.map(
    (channel, index) => channel * opacity + backgroundChannels[index] * (1 - opacity),
  );
  return `#${channels
    .map((channel) =>
      Math.round(channel * 255)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

describe("design token contrast contract", () => {
  it.each(p4RegressionPairs)(
    "$legacyForeground on $background passes as $foreground",
    ({ foreground, background }) => {
      expect(contrast(cssToken(foreground), cssToken(background))).toBeGreaterThanOrEqual(4.5);
    },
  );

  it.each(["--danger", "--danger-hover"])("keeps danger ink readable on %s", (background) => {
    expect(contrast(cssToken("--danger-ink"), cssToken(background))).toBeGreaterThanOrEqual(4.5);
  });

  it.each([
    "--status-neutral",
    "--status-info",
    "--status-warning",
    "--status-danger",
    "--status-success",
  ])("keeps white badge text readable on %s", (background) => {
    expect(contrast("#ffffff", cssToken(background))).toBeGreaterThanOrEqual(4.5);
  });

  it.each(["--sev-low", "--sev-critical", "--st-open", "--st-ack"])(
    "keeps %s readable over its 10% status surface",
    (foreground) => {
      const text = cssToken(foreground);
      const statusSurface = composite(text, cssToken("--panel"), 0.1);
      expect(contrast(text, statusSurface)).toBeGreaterThanOrEqual(4.5);
    },
  );
});

describe("semantic visual contract", () => {
  it.each([
    "--action-primary",
    "--action-primary-hover",
    "--action-primary-ink",
    "--action-secondary",
    "--action-secondary-hover",
    "--action-secondary-border",
    "--action-secondary-ink",
    "--action-danger",
    "--action-danger-hover",
    "--action-danger-ink",
    "--feedback-success",
    "--feedback-warning",
    "--feedback-danger",
    "--status-neutral",
    "--status-info",
    "--status-warning",
    "--status-danger",
    "--status-success",
  ])("defines %s", (token) => {
    expect(() => cssToken(token)).not.toThrow();
  });

  it("makes shared actions and feedback consume semantic roles", () => {
    expect(buttonSource).toContain("bg-action-primary");
    expect(buttonSource).toContain("bg-action-secondary");
    expect(buttonSource).toContain("bg-action-danger");
    expect(alertSource).toContain("text-feedback-success");
    expect(alertSource).toContain("text-feedback-warning");
    expect(alertSource).toContain("text-feedback-danger");
  });

  it.each([
    "--rel-created",
    "--rel-progress",
    "--rel-blocked",
    "--rel-completed",
    "--rel-cancelled",
  ])("defines %s", (token) => {
    expect(() => cssToken(token)).not.toThrow();
  });

  it("documents exactly five principal palette colors and their roles", () => {
    expect(designSystem).toContain("## Primary palette — 5 colors");
    expect(designSystem.match(/^\| `#[0-9A-F]{6}` \|/gm)).toHaveLength(5);
    expect(designSystem).toContain("Primary action");
    expect(designSystem).toContain("Secondary action");
    expect(designSystem).toContain("Success");
    expect(designSystem).toContain("Warning");
    expect(designSystem).toContain("Danger");
  });
});

describe("shape and border contract", () => {
  const radii = {
    "--ow-radius-sm": "3px",
    "--ow-radius-md": "6px",
    "--ow-radius-lg": "12px",
    "--ow-radius-full": "9999px",
  } as const;

  it.each(Object.entries(radii))("pins %s at %s", (token, value) => {
    expect(cssToken(token)).toBe(value);
  });

  /**
   * Three weights exist so a separator inside a surface, the edge of that
   * surface and a control the user can type into can be told apart. If they
   * ever collapse to the same value the vocabulary is gone, and every line in
   * the product reads with equal weight again.
   */
  it("keeps the three neutral border weights strictly ordered", () => {
    const alpha = (token: string) => {
      const percent = cssToken(token).match(/([\d.]+)%\s*\)/)?.[1];
      if (!percent) throw new Error(`${token} is not an alpha border`);
      return Number(percent);
    };
    expect(alpha("--ow-border-muted")).toBeLessThan(alpha("--ow-border"));
    expect(alpha("--ow-border")).toBeLessThan(alpha("--ow-border-emphasis"));
  });

  it("reserves the thick border width for focus, not for dividers", () => {
    expect(cssToken("--ow-border-width")).toBe("1px");
    expect(cssToken("--ow-border-width-thick")).toBe("2px");
    const divider = stylesheet.match(/\.scroll-divider \{[^}]+\}/)?.[0] ?? "";
    expect(divider).toContain("var(--ow-border-width)");
    expect(divider).not.toContain("--ow-border-width-thick");
  });

  it("draws the dialog divider only while its body can scroll", () => {
    expect(stylesheet).toContain("animation-timeline: scroll(self)");
    expect(stylesheet).toContain("calc(var(--ow-border-width) * var(--ow-can-scroll))");
  });

  /**
   * The utilities have to read the tokens, or the contract describes something
   * the interface does not use: 92 `rounded-md` consuming Tailwind's own 6px
   * would look identical today and drift the first time a token moves.
   */
  it("makes the radius utilities consume the tokens", () => {
    const config = readFileSync(resolve(process.cwd(), "tailwind.config.ts"), "utf8");
    const scale = config.match(/borderRadius: \{[^}]+\}/)?.[0] ?? "";
    expect(scale).toContain('sm: "var(--ow-radius-sm)"');
    expect(scale).toContain('DEFAULT: "var(--ow-radius-md)"');
    expect(scale).toContain('md: "var(--ow-radius-md)"');
    expect(scale).toContain('lg: "var(--ow-radius-lg)"');
    expect(scale).toContain('full: "var(--ow-radius-full)"');
  });

  it("documents the shape vocabulary in the design system contract", () => {
    expect(designSystem).toContain("## Shape and borders");
    for (const token of Object.keys(radii)) expect(designSystem).toContain(token);
    for (const token of ["--ow-border-muted", "--ow-border", "--ow-border-emphasis"]) {
      expect(designSystem).toContain(token);
    }
  });
});

describe("release lifecycle contract", () => {
  it("keeps --rel-progress readable on panels and over its 10% surface", () => {
    const text = cssToken("--rel-progress");
    expect(contrast(text, cssToken("--panel-2"))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(text, composite(text, cssToken("--panel"), 0.1))).toBeGreaterThanOrEqual(4.5);
  });

  it("separates a release in progress from an acknowledged incident", () => {
    // The overview shows both queues side by side. If these two resolved to the
    // same value, one colour would mean two things depending on which column
    // the operator happened to be reading.
    expect(cssToken("--rel-progress")).not.toBe(cssToken("--st-ack"));
  });

  it("dresses release states only from the release family", () => {
    // Domain components select a semantic tone and never style a badge from a
    // different domain's token family.
    const borrowed = releaseChipSource.match(/\b(?:text|bg|border)-(?:st|sev)-[a-z]+/g) ?? [];
    expect(borrowed).toEqual([]);
    expect(releaseChipSource).toContain('tone="info"');
  });

  it("keeps operational badges opaque and panel-shaped", () => {
    expect(statusBadgeSource).toContain('"inline-flex shrink-0 items-center rounded ');
    expect(statusBadgeSource).not.toContain("rounded-full");
    expect(statusBadgeSource).toContain("text-white");
    expect(statusBadgeSource).not.toMatch(/bg-status-[a-z]+\//);
  });
});

describe("platform preference contract", () => {
  it("defines shared reduced-motion and forced-colors policies", () => {
    expect(stylesheet).toMatch(/@media\s*\(prefers-reduced-motion:\s*reduce\)/);
    expect(stylesheet).toMatch(/@media\s*\(forced-colors:\s*active\)/);
    expect(stylesheet).toContain(".ow-progress-spinner");
    expect(stylesheet).toContain(".ow-action-menu-item");
  });
});
