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

  it("offers Activity only to a role that may read automation runs", () => {
    // Listing runs is Manager-only on the server, so a Responder or Observer
    // would be walking into a refusal.
    expect(
      primaryNavigationItems("team-1", true).map((item) => [item.labelKey, item.href]),
    ).toEqual([
      ["incidents", "/teams/team-1/incidents"],
      ["releases", "/teams/team-1/releases"],
      ["activity", "/teams/team-1/activity"],
      ["teams", "/teams/team-1/overview"],
    ]);
    expect(primaryNavigationItems("team-1", false).map((item) => item.labelKey)).not.toContain(
      "activity",
    );
  });

  it("does not offer Activity before a Team is known, whatever the capability", () => {
    expect(primaryNavigationItems(undefined, true).map((item) => item.labelKey)).toEqual(["teams"]);
  });
});

describe("isNavigationItemActive", () => {
  const [incidents, releases, activity, teams] = primaryNavigationItems("team-1", true);

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

  it("gives Activity its own place rather than folding it under Teams", () => {
    expect(isNavigationItemActive("/teams/team-1/activity", activity)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/activity", teams)).toBe(false);
    expect(isNavigationItemActive("/teams/team-1/automations", activity)).toBe(false);
  });

  it("keeps account settings distinct from Team settings", () => {
    expect(isNavigationItemActive("/settings", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/settings/connectors", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/settings", settingsNavigationItem)).toBe(false);
  });
});
