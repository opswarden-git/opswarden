import { describe, expect, it } from "vitest";
import { deriveIncidentHeaderActions } from "./incident-detail";

describe("deriveIncidentHeaderActions", () => {
  it.each([
    [["acknowledged"], { primary: "acknowledged", secondary: null }],
    [["escalated", "resolved"], { primary: "escalated", secondary: "resolved" }],
    [["resolved"], { primary: "resolved", secondary: null }],
    [[], { primary: null, secondary: null }],
  ] as const)("maps %j to one dominant action", (transitions, expected) => {
    expect(deriveIncidentHeaderActions([...transitions])).toEqual(expected);
  });
});
