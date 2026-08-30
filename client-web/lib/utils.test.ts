import { describe, expect, it } from "vitest";
import {
  cn,
  formatDateTime,
  formatDateOnly,
  formatDurationMs,
  formatFallback,
  formatRelativeAge,
  formatShortId,
  formatTimeOnly,
} from "./utils";

describe("lib/utils presentation primitives (WEB-020)", () => {
  it("merges tailwind class names correctly", () => {
    expect(cn("px-2 py-1", "bg-red-500", { "text-white": true })).toBe(
      "px-2 py-1 bg-red-500 text-white",
    );
  });

  it("formats relative age", () => {
    const past = new Date(Date.now() - 3600 * 1000).toISOString();
    expect(formatRelativeAge(past, "en")).toContain("hour");
  });

  it("formats date and time", () => {
    const date = "2026-08-14T12:00:00.000Z";
    expect(formatDateTime(date, "en")).toBeTruthy();
    expect(formatDateOnly(date, "en")).toBeTruthy();
    expect(formatTimeOnly(date, "en")).toBeTruthy();
  });

  it("handles invalid dates with fallback", () => {
    expect(formatDateTime("invalid-date", "en")).toBe("—");
  });

  it("formats duration in ms and seconds", () => {
    expect(formatDurationMs(450)).toBe("450ms");
    expect(formatDurationMs(2000)).toBe("2s");
    expect(formatDurationMs(2450)).toBe("2.5s");
    expect(formatDurationMs(-10)).toBe("0ms");
  });

  it("formats short IDs", () => {
    expect(formatShortId("12345678-abcd-efgh-ijkl")).toBe("12345678");
    expect(formatShortId("abc", 8)).toBe("abc");
    expect(formatShortId(null)).toBe("—");
    expect(formatShortId(undefined)).toBe("—");
  });

  it("formats fallback strings", () => {
    expect(formatFallback("hello")).toBe("hello");
    expect(formatFallback("")).toBe("—");
    expect(formatFallback(null, "N/A")).toBe("N/A");
    expect(formatFallback("   ")).toBe("—");
  });
});
