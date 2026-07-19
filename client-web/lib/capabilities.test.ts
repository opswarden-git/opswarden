import { describe, expect, it } from "vitest";
import contract from "../../contracts/role-capabilities.json";
import { deriveCapabilities, deriveIncidentActions, type TeamRole } from "./capabilities";

describe("deriveCapabilities", () => {
  it.each(["observer", "responder", "manager"] satisfies TeamRole[])(
    "matches the shared contract for %s",
    (role) => {
      expect(deriveCapabilities(role)).toEqual(contract[role]);
    },
  );
});

describe("deriveIncidentActions", () => {
  it("keeps observers read-only while preserving reactions", () => {
    expect(deriveIncidentActions("observer", "acknowledged")).toEqual({
      canAssign: false,
      canDelete: false,
      canWriteTimeline: false,
      canReact: true,
      transitions: [],
    });
  });

  it("offers responders only the transitions valid for the current state", () => {
    expect(deriveIncidentActions("responder", "open").transitions).toEqual(["acknowledged"]);
    expect(deriveIncidentActions("responder", "acknowledged").transitions).toEqual([
      "escalated",
      "resolved",
    ]);
    expect(deriveIncidentActions("responder", "escalated").transitions).toEqual(["resolved"]);
    expect(deriveIncidentActions("responder", "resolved").transitions).toEqual([]);
  });

  it("reserves assignment and deletion for managers", () => {
    expect(deriveIncidentActions("manager", "open")).toMatchObject({
      canAssign: true,
      canDelete: true,
    });
    expect(deriveIncidentActions("manager", "resolved")).toMatchObject({
      canAssign: false,
      canDelete: true,
    });
  });
});
