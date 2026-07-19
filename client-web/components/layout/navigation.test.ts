import { describe, expect, it } from "vitest";
import { primaryNavigationItems } from "./navigation";

describe("primaryNavigationItems", () => {
  it("does not invent an Incidents link before a Team is known", () => {
    expect(primaryNavigationItems().map((item) => item.labelKey)).toEqual(["teams"]);
  });

  it("creates distinct Team-scoped links once the Team is known", () => {
    const items = primaryNavigationItems("team-1");

    expect(items.map((item) => [item.labelKey, item.href])).toEqual([
      ["incidents", "/teams/team-1/incidents"],
      ["releases", "/teams/team-1/releases"],
      ["teams", "/teams/team-1/overview"],
    ]);
    expect(new Set(items.map((item) => item.labelKey)).size).toBe(items.length);
  });
});
