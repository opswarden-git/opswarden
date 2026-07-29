import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AutomationRule, AutomationService, TeamConnection } from "@/lib/queries/automations";
import { RulesView } from "./RulesView";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values?.name ? `${key}:${values.name}` : key,
}));

const createMutation = {
  error: null,
  isPending: false,
  mutate: vi.fn(),
};
const updateMutation = {
  error: null,
  isPending: false,
  mutate: vi.fn(),
};
const deleteMutation = {
  error: null,
  isPending: false,
  mutate: vi.fn(),
};

vi.mock("@/lib/queries/automations", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/queries/automations")>();
  return {
    ...actual,
    useCreateAutomationRule: () => createMutation,
    useUpdateAutomationRule: () => updateMutation,
    useDeleteAutomationRule: () => deleteMutation,
  };
});

const catalog: AutomationService[] = [
  {
    name: "github",
    label: "GitHub",
    connection: null,
    actions: [
      {
        name: "github_ci_failed",
        label: "CI failed",
        description: "A workflow failed",
        connection_service: "github",
        fields: [
          {
            name: "branch",
            label: "Branch",
            description: "Optional branch filter",
            input_type: "text",
            required: false,
            default_value: null,
            options: [],
          },
        ],
      },
    ],
    reactions: [],
  },
  {
    name: "opswarden",
    label: "OpsWarden",
    connection: null,
    actions: [
      {
        name: "release_created",
        label: "Release created",
        description: "A release was created",
        connection_service: "opswarden",
        fields: [],
      },
    ],
    reactions: [
      {
        name: "create_incident",
        label: "Create incident",
        description: "Create a high incident",
        connection_service: null,
        fields: [
          {
            name: "severity",
            label: "Severity",
            description: "Incident severity",
            input_type: "select",
            required: true,
            default_value: "high",
            options: [
              { value: "high", label: "High" },
              { value: "critical", label: "Critical" },
            ],
          },
        ],
      },
    ],
  },
];

const connection: TeamConnection = {
  id: "connection-github",
  team_id: "team-1",
  service: "github",
  secret_configured: true,
  token_configured: false,
  oauth_configured: false,
  oauth_refresh_configured: false,
  endpoint_configured: false,
  created_at: "2026-07-25T10:00:00Z",
  updated_at: "2026-07-25T10:00:00Z",
  verified_at: null,
  last_delivery_at: null,
  last_error_code: null,
  webhook_path: "/webhooks/github/connection-github",
};

const opswardenConnection: TeamConnection = {
  ...connection,
  id: "connection-opswarden",
  service: "opswarden",
  secret_configured: false,
  webhook_path: null,
};

const rule: AutomationRule = {
  id: "rule-1",
  team_id: "team-1",
  name: "Failed CI",
  trigger_connection_id: connection.id,
  trigger_kind: "github_ci_failed",
  trigger_config: { branch: "main" },
  reaction_kind: "create_incident",
  reaction_connection_id: null,
  reaction_config: { severity: "high" },
  enabled: true,
  created_by: "user-1",
  created_at: "2026-07-25T10:00:00Z",
  updated_at: "2026-07-25T10:00:00Z",
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("RulesView", () => {
  it("creates a catalog-driven rule from the empty state", () => {
    const setIsCreatingRule = vi.fn();
    render(
      <RulesView
        catalog={catalog}
        connections={[connection]}
        rules={[]}
        teamId="team-1"
        isCreatingRule
        setIsCreatingRule={setIsCreatingRule}
      />,
    );

    expect(screen.getByText("noRules")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("ruleName"), { target: { value: "Failed main CI" } });
    fireEvent.change(screen.getByLabelText("sourceConnection"), {
      target: { value: connection.id },
    });
    fireEvent.change(screen.getByRole("textbox", { name: /Branch/ }), {
      target: { value: "main" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: /Severity/ }), {
      target: { value: "critical" },
    });
    fireEvent.click(screen.getByRole("button", { name: "createRule" }));

    expect(createMutation.mutate).toHaveBeenCalledWith(
      {
        name: "Failed main CI",
        trigger_connection_id: connection.id,
        trigger_kind: "github_ci_failed",
        trigger_config: { branch: "main" },
        reaction_kind: "create_incident",
        reaction_connection_id: null,
        reaction_config: { severity: "critical" },
      },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });

  it("creates a rule from a native OpsWarden event without exposing a connection", () => {
    render(
      <RulesView
        catalog={catalog}
        connections={[connection, opswardenConnection]}
        rules={[]}
        teamId="team-1"
        isCreatingRule
        setIsCreatingRule={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("ruleName"), {
      target: { value: "Release opens incident" },
    });
    fireEvent.change(screen.getByLabelText("event"), {
      target: { value: "release_created" },
    });
    expect(screen.queryByLabelText("sourceConnection")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "createRule" }));

    expect(createMutation.mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Release opens incident",
        trigger_connection_id: opswardenConnection.id,
        trigger_kind: "release_created",
        reaction_kind: "create_incident",
      }),
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });

  it("renders both responsive rule projections and toggles a rule", async () => {
    render(
      <RulesView
        catalog={catalog}
        connections={[connection]}
        rules={[rule]}
        teamId="team-1"
        isCreatingRule={false}
        setIsCreatingRule={vi.fn()}
      />,
    );

    expect(screen.getAllByText("Failed CI")).toHaveLength(2);
    const table = screen.getByRole("table", { name: "rulesList" });
    expect(within(table).getByText("CI failed")).toBeInTheDocument();
    fireEvent.pointerDown(screen.getAllByRole("button", { name: "actionsMenu" })[0], {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "disable" }));
    expect(updateMutation.mutate).toHaveBeenCalledWith({ ruleId: "rule-1", enabled: false });
  });

  it("opens the edit form with persisted catalog values", async () => {
    render(
      <RulesView
        catalog={catalog}
        connections={[connection]}
        rules={[rule]}
        teamId="team-1"
        isCreatingRule={false}
        setIsCreatingRule={vi.fn()}
      />,
    );

    fireEvent.pointerDown(screen.getAllByRole("button", { name: "actionsMenu" })[0], {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(await screen.findByRole("menuitem", { name: "edit" }));
    expect(screen.getByRole("dialog", { name: "editRule" })).toBeInTheDocument();
    expect(screen.getByLabelText("ruleName")).toHaveValue("Failed CI");
    expect(screen.getByRole("textbox", { name: /Branch/ })).toHaveValue("main");
  });
});
