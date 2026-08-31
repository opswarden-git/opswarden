import { describe, expect, it } from "vitest";
import { projectRule } from "./RulesView";

const rule = {
  id: "rule-1",
  team_id: "team-1",
  name: "Failed CI",
  enabled: true,
  trigger_connection_id: "conn-1",
  trigger_kind: "github_ci_failed",
  trigger_config: { branch: "main" },
  reaction_kind: "create_incident",
  reaction_connection_id: null,
  reaction_config: { severity: "high" },
  created_by: "user-1",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  next_run_at: null,
};

describe("projectRule", () => {
  it("projects rule capability labels and formatted dates", () => {
    const actions = [
      {
        name: "github_ci_failed",
        label: "CI failed",
        description: "CI failed",
        connection_service: "github",
        fields: [],
        service: "github",
        builtIn: false,
      },
    ];
    const reactions = [
      {
        name: "create_incident",
        label: "Create Incident",
        description: "Create Incident",
        connection_service: "opswarden",
        fields: [],
        service: "opswarden",
        builtIn: true,
      },
    ];

    const projected = projectRule(rule, actions, reactions, "en", "Disabled");
    expect(projected.rule).toBe(rule);
    expect(projected.triggerLabel).toBe("CI failed");
    expect(projected.reactionLabel).toBe("Create Incident");
    expect(projected.nextRunLabel).toBe("—");
    expect(projected.updatedAtLabel).toContain("2026");

    const disabledProjected = projectRule(
      { ...rule, enabled: false },
      actions,
      reactions,
      "en",
      "Disabled",
    );
    expect(disabledProjected.nextRunLabel).toBe("Disabled");
  });
});
