import { describe, expect, it } from "vitest";
import type { ReleaseListItem, ReleaseState } from "@/lib/queries/releases";
import { normalizeReleaseView, releaseBelongsToView, releaseViewCounts } from "./release-views";

function release(state: ReleaseState): ReleaseListItem {
  return {
    release_id: state,
    team_id: "team-1",
    title: state,
    state,
    progress: { completed: 0, total: 2 },
    next_step: { position: 0, name: "build" },
    blockers: [],
    linked_incident_ids: [],
    created_at: "2026-07-18T00:00:00Z",
    updated_at: "2026-07-18T00:00:00Z",
  };
}

describe("release views", () => {
  const releases = [
    release("created"),
    release("in_progress"),
    release("blocked"),
    release("completed"),
    release("cancelled"),
  ];

  it("keeps Active focused on executable work and separates blocked releases", () => {
    expect(releases.filter((item) => releaseBelongsToView(item, "active"))).toHaveLength(2);
    expect(releases.filter((item) => releaseBelongsToView(item, "blocked"))).toEqual([
      expect.objectContaining({ state: "blocked" }),
    ]);
  });

  it("returns stable counters for every URL-backed view", () => {
    expect(releaseViewCounts(releases)).toEqual({
      active: 2,
      blocked: 1,
      completed: 1,
      cancelled: 1,
      all: 5,
    });
  });

  it("falls back to Active for missing or unknown URL values", () => {
    expect(normalizeReleaseView(null)).toBe("active");
    expect(normalizeReleaseView("future-view")).toBe("active");
    expect(normalizeReleaseView("completed")).toBe("completed");
  });
});
