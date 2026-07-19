import { describe, expect, it } from "vitest";
import { parseTeamPath, pathForTeamSwitch, teamPath } from "./team-routing";

describe("team routing", () => {
  it("builds and parses canonical Team URLs", () => {
    const path = teamPath("team-1", "incidents", "incident-9");

    expect(path).toBe("/teams/team-1/incidents/incident-9");
    expect(parseTeamPath(path)).toEqual({
      teamId: "team-1",
      section: "incidents",
      resourceId: "incident-9",
    });
  });

  it("preserves the product area when switching Team", () => {
    expect(pathForTeamSwitch("/teams/team-1/releases", "team-2")).toBe("/teams/team-2/releases");
    expect(pathForTeamSwitch("/teams/team-1/members", "team-2")).toBe("/teams/team-2/members");
    expect(pathForTeamSwitch("/teams/team-1/automations", "team-2")).toBe(
      "/teams/team-2/automations",
    );
  });

  it("drops a resource that cannot belong to the next Team", () => {
    expect(pathForTeamSwitch("/teams/team-1/incidents/incident-9", "team-2")).toBe(
      "/teams/team-2/incidents",
    );
  });

  it("treats a Release id as a Team-scoped resource", () => {
    expect(parseTeamPath("/teams/team-1/releases/release-7")).toEqual({
      teamId: "team-1",
      section: "releases",
      resourceId: "release-7",
    });
    expect(pathForTeamSwitch("/teams/team-1/releases/release-7", "team-2")).toBe(
      "/teams/team-2/releases",
    );
  });

  it("uses Incidents when switching from a global page", () => {
    expect(pathForTeamSwitch("/settings", "team-2")).toBe("/teams/team-2/incidents");
  });
});
