import { describe, expect, it } from "vitest";
import {
  isNavigationItemActive,
  primaryNavigationItems,
  settingsNavigationItem,
} from "./navigation";

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

describe("isNavigationItemActive", () => {
  const [incidents, releases, teams] = primaryNavigationItems("team-1");

  it("keeps Team resource details attached to their collection", () => {
    expect(isNavigationItemActive("/teams/team-1/incidents/incident-1", incidents)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/incidents/incident-1", releases)).toBe(false);
    expect(isNavigationItemActive("/teams/team-1/releases/release-1", releases)).toBe(true);
  });

  it("groups the Team workspace sections under Teams", () => {
    for (const section of ["overview", "members", "automations", "settings"]) {
      expect(isNavigationItemActive(`/teams/team-1/${section}`, teams)).toBe(true);
    }

    expect(isNavigationItemActive("/teams", teams)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/incidents", teams)).toBe(false);
  });

  it("keeps account settings distinct from Team settings", () => {
    expect(isNavigationItemActive("/settings", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/settings/connectors", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/settings", settingsNavigationItem)).toBe(false);
  });
});
