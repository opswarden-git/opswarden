import { describe, expect, it } from "vitest";
import type { IncidentActivityItem } from "@/lib/queries/incidents";
import { GROUPING_WINDOW_MS, groupsWithPrevious, resolveGrouping } from "./activity-grouping";

const BASE = new Date("2026-08-06T12:00:00.000Z").getTime();

function at(offsetMs: number) {
  return new Date(BASE + offsetMs).toISOString();
}

function note(offsetMs: number, authorId: string | null, content = "note"): IncidentActivityItem {
  return {
    type: "human_note",
    entry_id: `entry-${offsetMs}`,
    author: authorId ? { user_id: authorId, email: `${authorId}@opswarden.local` } : null,
    content,
    created_at: at(offsetMs),
    edited_at: null,
    attachments: [],
    reactions: [],
  };
}

function event(
  offsetMs: number,
  kind: Extract<IncidentActivityItem, { type: "system_event" }>["kind"],
): IncidentActivityItem {
  return {
    type: "system_event",
    id: `event-${offsetMs}`,
    kind,
    actor: null,
    subject: null,
    data: {},
    created_at: at(offsetMs),
  };
}

describe("consecutive note grouping", () => {
  it("merges notes from the same author inside the window", () => {
    expect(groupsWithPrevious(note(60_000, "ada"), note(0, "ada"))).toBe(true);
  });

  it("pins the window at two minutes rather than Mattermost's five", () => {
    // Stated as a literal on purpose: the departure from POST_COLLAPSE_TIMEOUT
    // is the decision, so widening it back must fail here rather than pass
    // quietly against a constant that moved with it.
    expect(GROUPING_WINDOW_MS).toBe(120_000);
  });

  it("holds the block at exactly two minutes and opens a new one past it", () => {
    expect(groupsWithPrevious(note(120_000, "ada"), note(0, "ada"))).toBe(true);
    expect(groupsWithPrevious(note(120_001, "ada"), note(0, "ada"))).toBe(false);
    expect(groupsWithPrevious(note(180_000, "ada"), note(0, "ada"))).toBe(false);
  });

  it("never merges different authors", () => {
    expect(groupsWithPrevious(note(1_000, "grace"), note(0, "ada"))).toBe(false);
  });

  it("never merges deleted accounts, which are not an identity", () => {
    expect(groupsWithPrevious(note(1_000, null), note(0, null))).toBe(false);
  });

  it("never merges the first item", () => {
    expect(groupsWithPrevious(note(0, "ada"), undefined)).toBe(false);
  });
});

describe("system events break the run", () => {
  // The D11 exclusions transposed from Mattermost: a webhook post, a system
  // message and a priority post never join a block. In OpsWarden all three are
  // system events, so excluding the type covers every case at once.
  it.each([
    ["created" as const],
    ["status_changed" as const],
    ["assigned" as const],
    ["severity_changed" as const],
  ])("a %s event is never absorbed into a block of notes", (kind) => {
    expect(groupsWithPrevious(event(1_000, kind), note(0, "ada"))).toBe(false);
  });

  it("reopens a block after an escalation, even for the same author", () => {
    const items = [note(0, "ada"), event(1_000, "status_changed"), note(2_000, "ada")];
    const grouping = resolveGrouping(items);

    // The note after the escalation starts a fresh block: it must carry its own
    // name and timestamp, or the escalation reads as part of the conversation.
    expect(grouping[2].continuesAbove).toBe(false);
  });
});

describe("resolveGrouping", () => {
  it("marks the seam on both sides of a run", () => {
    const grouping = resolveGrouping([note(0, "ada"), note(1_000, "ada"), note(2_000, "ada")]);

    expect(grouping.map((entry) => entry.continuesAbove)).toEqual([false, true, true]);
    expect(grouping.map((entry) => entry.continuesBelow)).toEqual([true, true, false]);
  });

  it("closes a block when the next author speaks", () => {
    const grouping = resolveGrouping([note(0, "ada"), note(1_000, "grace")]);

    expect(grouping[0].continuesBelow).toBe(false);
    expect(grouping[1].continuesAbove).toBe(false);
  });

  it("handles an empty transcript", () => {
    expect(resolveGrouping([])).toEqual([]);
  });
});
