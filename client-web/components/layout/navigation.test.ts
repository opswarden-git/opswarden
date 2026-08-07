import { describe, expect, it } from "vitest";
import {
  isNavigationItemActive,
  navigationTree,
  primaryNavigationItems,
  settingsNavigationItem,
} from "./navigation";

describe("navigationTree", () => {
  it("does not invent Team links before a Team is known", () => {
    expect(navigationTree().map((node) => node.labelKey)).toEqual(["teams"]);
  });

  it("keeps shift work flat and folds configuration into one branch", () => {
    // The shape both IRIS and TheHive settled on: what you do during a shift is
    // never nested, what you configure between shifts always is.
    expect(navigationTree("team-1").map((node) => [node.kind, node.labelKey])).toEqual([
      ["leaf", "overview"],
      ["leaf", "incidents"],
      ["leaf", "releases"],
      ["branch", "manage"],
    ]);
  });

  it("only exposes Manager-only activity when its capability is known", () => {
    expect(navigationTree("team-1").map((node) => node.labelKey)).not.toContain("activity");
    expect(navigationTree("team-1", true).map((node) => node.labelKey)).toContain("activity");
  });

  it("puts the landing page above the branch, never inside it", () => {
    const branch = navigationTree("team-1").find((node) => node.kind === "branch");
    expect(branch?.kind === "branch" && branch.children.map((child) => child.labelKey)).toEqual([
      "members",
      "automations",
      "teamSettings",
    ]);
  });

  it("reaches every destination exactly once when flattened", () => {
    const items = primaryNavigationItems("team-1");

    expect(items.map((item) => [item.labelKey, item.href])).toEqual([
      ["overview", "/teams/team-1/overview"],
      ["incidents", "/teams/team-1/incidents"],
      ["releases", "/teams/team-1/releases"],
      ["members", "/teams/team-1/members"],
      ["automations", "/teams/team-1/automations"],
      ["teamSettings", "/teams/team-1/settings"],
    ]);
    expect(new Set(items.map((item) => item.labelKey)).size).toBe(items.length);
  });

  it("includes activity in the flat mobile view for Managers", () => {
    expect(primaryNavigationItems("team-1", true).map((item) => item.labelKey)).toContain(
      "activity",
    );
  });
});

describe("isNavigationItemActive", () => {
  const [overview, incidents, releases, members] = primaryNavigationItems("team-1");

  it("keeps Team resource details attached to their collection", () => {
    expect(isNavigationItemActive("/teams/team-1/incidents/incident-1", incidents)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/incidents/incident-1", releases)).toBe(false);
    expect(isNavigationItemActive("/teams/team-1/releases/release-1", releases)).toBe(true);
  });

  it("gives each section its own entry rather than one that swallows the rest", () => {
    // The previous shape lit "Teams" for overview, members, automations and
    // settings alike, so the sidebar could not say which page you were on.
    expect(isNavigationItemActive("/teams/team-1/overview", overview)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/members", overview)).toBe(false);
    expect(isNavigationItemActive("/teams/team-1/members", members)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/incidents", overview)).toBe(false);
  });

  it("keeps account settings distinct from Team settings", () => {
    expect(isNavigationItemActive("/settings", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/settings/connectors", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/settings", settingsNavigationItem)).toBe(false);
  });
});
