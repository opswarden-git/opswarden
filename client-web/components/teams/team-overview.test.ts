import { describe, expect, it } from "vitest";
import type { IncidentListItem } from "@/lib/queries/incidents";
import type { ReleaseListItem } from "@/lib/queries/releases";
import { deriveTeamOverview } from "./team-overview";

const incident = (
  overrides: Partial<IncidentListItem> & Pick<IncidentListItem, "id" | "title">,
): IncidentListItem => ({
  team_id: "team-1",
  description: "",
  status: "open",
  severity: "high",
  assignee: null,
  created_at: "2026-07-18T10:00:00Z",
  created_by: null,
  updated_at: "2026-07-18T11:00:00Z",
  ...overrides,
});

const release = (
  overrides: Partial<ReleaseListItem> & Pick<ReleaseListItem, "release_id" | "title">,
): ReleaseListItem => ({
  team_id: "team-1",
  state: "created",
  progress: { completed: 0, total: 2 },
  next_step: { position: 0, name: "Build" },
  blockers: [],
  linked_incident_ids: [],
  created_at: "2026-07-18T09:00:00Z",
  updated_at: "2026-07-18T09:00:00Z",
  ...overrides,
});

const incidents = [
  incident({ id: "mine", title: "Mine", assignee: { user_id: "me", email: "me@test" } }),
  incident({ id: "unassigned", title: "Unassigned", severity: "medium" }),
  incident({ id: "escalated", title: "Escalated", status: "escalated", severity: "critical" }),
  incident({ id: "done", title: "Done", status: "resolved" }),
];
const releases = [
  release({
    release_id: "blocked",
    title: "Blocked",
    state: "blocked",
    blockers: [
      { incident_id: "escalated", title: "Escalated", status: "escalated", severity: "critical" },
    ],
  }),
  release({ release_id: "ready", title: "Ready" }),
  release({ release_id: "complete", title: "Complete", state: "completed", next_step: null }),
];

describe("deriveTeamOverview", () => {
  it("builds one ranked inter-resource attention list without terminal work", () => {
    const result = deriveTeamOverview({
      incidents,
      releases,
      role: "responder",
      userId: "me",
      canProgressRelease: true,
    });

    expect(result.attention.map((item) => item.id)).toEqual([
      "blocked",
      "mine",
      "escalated",
      "unassigned",
      "ready",
    ]);
    expect(result.attention.some((item) => item.id === "done")).toBe(false);
    expect(result.attention.some((item) => item.id === "complete")).toBe(false);
    expect(result.counts).toEqual({
      active: 3,
      unacknowledged: 2,
      assignedToMe: 1,
      escalated: 1,
      blockedReleases: 1,
    });
  });

  it("gives Managers an explicit unassigned reason", () => {
    const result = deriveTeamOverview({
      incidents,
      releases: [],
      role: "manager",
      userId: "manager",
      canProgressRelease: true,
    });

    expect(result.attention.find((item) => item.id === "unassigned")?.reason).toBe(
      "unassignedUnacknowledged",
    );
  });

  it("keeps Observer attention read-only by omitting executable Release steps", () => {
    const result = deriveTeamOverview({
      incidents,
      releases,
      role: "observer",
      userId: "observer",
      canProgressRelease: false,
    });

    expect(result.attention.some((item) => item.id === "blocked")).toBe(true);
    expect(result.attention.some((item) => item.id === "ready")).toBe(false);
  });

  it("keeps one executable Release visible when Incidents fill the attention limit", () => {
    const crowdedIncidents = Array.from({ length: 9 }, (_, index) =>
      incident({
        id: `incident-${index}`,
        title: `Incident ${index}`,
        severity: index < 3 ? "critical" : "high",
      }),
    );
    const result = deriveTeamOverview({
      incidents: crowdedIncidents,
      releases: [release({ release_id: "ready-release", title: "Ready release" })],
      role: "responder",
      userId: "me",
      canProgressRelease: true,
    });

    expect(result.attention).toHaveLength(7);
    expect(result.attention.some((item) => item.id === "ready-release")).toBe(true);
    expect(result.attention.filter((item) => item.severity === "critical")).toHaveLength(3);
  });
});
