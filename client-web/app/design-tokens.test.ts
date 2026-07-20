import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesheet = readFileSync(resolve(process.cwd(), "app/globals.css"), "utf8");

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

function cssToken(name: string) {
  const match = stylesheet.match(new RegExp(`${name}:\\s*(#[0-9a-f]{3,8})`, "i"));
  if (!match) throw new Error(`Missing opaque token ${name}`);
  return match[1];
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

  it.each(["--sev-low", "--sev-critical", "--st-open", "--st-ack"])(
    "keeps %s readable over its 10% status surface",
    (foreground) => {
      const text = cssToken(foreground);
      const statusSurface = composite(text, cssToken("--panel"), 0.1);
      expect(contrast(text, statusSurface)).toBeGreaterThanOrEqual(4.5);
    },
  );
});

describe("platform preference contract", () => {
  it("defines shared reduced-motion and forced-colors policies", () => {
    expect(stylesheet).toMatch(/@media\s*\(prefers-reduced-motion:\s*reduce\)/);
    expect(stylesheet).toMatch(/@media\s*\(forced-colors:\s*active\)/);
    expect(stylesheet).toContain(".ow-progress-spinner");
    expect(stylesheet).toContain(".ow-action-menu-item");
  });
});
