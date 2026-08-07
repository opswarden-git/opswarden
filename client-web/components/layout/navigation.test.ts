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

  it("groups every Team destination without hiding any of them", () => {
    expect(navigationTree("team-1").map((node) => [node.kind, node.labelKey])).toEqual([
      ["branch", "operations"],
      ["branch", "manage"],
    ]);
  });

  it("only exposes Manager-only automation configuration when its capability is known", () => {
    expect(primaryNavigationItems("team-1").map((item) => item.labelKey)).not.toContain("rules");
    expect(primaryNavigationItems("team-1", true).map((item) => item.labelKey)).toEqual(
      expect.arrayContaining(["rules", "integrations"]),
    );
  });

  it("keeps operations and administration as explicit groups", () => {
    const groups = navigationTree("team-1", true).filter((node) => node.kind === "branch");
    expect(groups.map((group) => group.children.map((child) => child.labelKey))).toEqual([
      ["overview", "incidents", "releases", "rules", "integrations"],
      ["members", "teamSettings"],
    ]);
  });

  it("reaches every destination exactly once when flattened", () => {
    const items = primaryNavigationItems("team-1", true);

    expect(items.map((item) => [item.labelKey, item.href])).toEqual([
      ["overview", "/teams/team-1/overview"],
      ["incidents", "/teams/team-1/incidents"],
      ["releases", "/teams/team-1/releases"],
      ["rules", "/teams/team-1/automations"],
      ["integrations", "/teams/team-1/automations?view=connections"],
      ["members", "/teams/team-1/members"],
      ["teamSettings", "/teams/team-1/settings"],
    ]);
    expect(new Set(items.map((item) => item.labelKey)).size).toBe(items.length);
  });

  it("keeps run history contextual instead of exposing Activity", () => {
    const labels = primaryNavigationItems("team-1", true).map((item) => item.labelKey);
    expect(labels).not.toContain("activity");
    expect(labels).toEqual(expect.arrayContaining(["rules", "integrations"]));
  });

  it("does not lead non-Managers to Manager-only administration", () => {
    expect(primaryNavigationItems("team-1").map((item) => item.labelKey)).toEqual([
      "overview",
      "incidents",
      "releases",
      "members",
    ]);
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

  it("distinguishes direct Rules and Integrations destinations on the shared route", () => {
    const items = primaryNavigationItems("team-1", true);
    const rules = items.find((item) => item.labelKey === "rules")!;
    const integrations = items.find((item) => item.labelKey === "integrations")!;

    expect(isNavigationItemActive("/teams/team-1/automations", rules, new URLSearchParams())).toBe(
      true,
    );
    expect(
      isNavigationItemActive(
        "/teams/team-1/automations",
        integrations,
        new URLSearchParams("view=connections"),
      ),
    ).toBe(true);
    expect(
      isNavigationItemActive(
        "/teams/team-1/automations",
        rules,
        new URLSearchParams("view=connections"),
      ),
    ).toBe(false);
    expect(
      isNavigationItemActive("/teams/team-1/automations", rules, new URLSearchParams("view=runs")),
    ).toBe(true);
  });

  it("keeps account settings distinct from Team settings", () => {
    expect(isNavigationItemActive("/settings", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/settings/connectors", settingsNavigationItem)).toBe(true);
    expect(isNavigationItemActive("/teams/team-1/settings", settingsNavigationItem)).toBe(false);
  });
});
