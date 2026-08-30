import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AutomationRule, AutomationService, TeamConnection } from "@/lib/queries/automations";
import { projectRule, RulesView } from "./RulesView";

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
    connection: {
      description: "GitHub webhook",
      fields: [],
      oauth: null,
      testable: false,
    },
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
    name: "alertmanager",
    label: "Alertmanager",
    connection: {
      description: "Authenticated Alertmanager webhook",
      fields: [],
      oauth: null,
      testable: false,
    },
    actions: [
      {
        name: "alert_firing",
        label: "Alert firing",
        description: "One Alertmanager alert started firing",
        connection_service: "alertmanager",
        fields: [],
      },
      {
        name: "alert_resolved",
        label: "Alert resolved",
        description: "One Alertmanager alert was resolved",
        connection_service: "alertmanager",
        fields: [],
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
  {
    name: "timer",
    label: "Timer",
    connection: null,
    actions: [
      {
        name: "daily_at",
        label: "Every day",
        description: "Run every day",
        connection_service: "timer",
        fields: [
          {
            name: "time",
            label: "Local time",
            description: "HH:MM",
            input_type: "time",
            required: true,
            default_value: "09:00",
            options: [],
          },
          {
            name: "timezone",
            label: "Timezone",
            description: "IANA timezone",
            input_type: "text",
            required: true,
            default_value: "Europe/Paris",
            options: [],
          },
        ],
      },
    ],
    reactions: [],
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

const alertmanagerConnection: TeamConnection = {
  ...connection,
  id: "connection-alertmanager",
  service: "alertmanager",
  webhook_path: "/webhooks/alertmanager/connection-alertmanager",
};

const timerConnection: TeamConnection = {
  ...connection,
  id: "connection-timer",
  service: "timer",
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
  next_run_at: null,
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

  it("keeps same-named provider events distinct", () => {
    const github = {
      ...catalog[0],
      actions: [{ ...catalog[0].actions[0], name: "ci_failed", label: "GitHub CI failed" }],
    };
    const gitlab: AutomationService = {
      ...github,
      name: "gitlab",
      label: "GitLab",
      actions: [
        {
          ...github.actions[0],
          label: "GitLab pipeline failed",
          connection_service: "gitlab",
        },
      ],
    };
    const gitlabConnection = {
      ...connection,
      id: "connection-gitlab",
      service: "gitlab",
    };

    render(
      <RulesView
        catalog={[github, gitlab, catalog[2]]}
        connections={[connection, gitlabConnection]}
        rules={[]}
        teamId="team-1"
        isCreatingRule
        setIsCreatingRule={vi.fn()}
      />,
    );

    const eventSelect = screen.getByLabelText("event");
    expect(within(eventSelect).getByRole("option", { name: "GitHub CI failed" })).toHaveValue(
      "github:ci_failed",
    );
    expect(within(eventSelect).getByRole("option", { name: "GitLab pipeline failed" })).toHaveValue(
      "gitlab:ci_failed",
    );

    fireEvent.change(eventSelect, { target: { value: "gitlab:ci_failed" } });
    expect(
      within(screen.getByLabelText("sourceConnection")).getByRole("option", {
        name: "gitlab · connecti",
      }),
    ).toHaveValue(gitlabConnection.id);
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

  it("creates a daily Timer rule through its internal connection", () => {
    render(
      <RulesView
        catalog={catalog}
        connections={[connection, opswardenConnection, timerConnection]}
        rules={[]}
        teamId="team-1"
        isCreatingRule
        setIsCreatingRule={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("ruleName"), {
      target: { value: "Daily handover" },
    });
    fireEvent.change(screen.getByLabelText("event"), { target: { value: "daily_at" } });
    expect(screen.queryByLabelText("sourceConnection")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(/Local time/), { target: { value: "09:30" } });
    fireEvent.change(screen.getByLabelText(/Timezone/), { target: { value: "UTC" } });
    fireEvent.click(screen.getByRole("button", { name: "createRule" }));

    expect(createMutation.mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Daily handover",
        trigger_connection_id: timerConnection.id,
        trigger_kind: "daily_at",
        trigger_config: { time: "09:30", timezone: "UTC" },
      }),
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });

  it.each([
    ["alert_firing", "Open incident for firing alert"],
    ["alert_resolved", "Notify when alert resolves"],
  ])("creates an accessible Alertmanager %s rule", (triggerKind, ruleName) => {
    render(
      <RulesView
        catalog={catalog}
        connections={[connection, alertmanagerConnection]}
        rules={[]}
        teamId="team-1"
        isCreatingRule
        setIsCreatingRule={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("ruleName"), { target: { value: ruleName } });
    fireEvent.change(screen.getByLabelText("event"), { target: { value: triggerKind } });
    fireEvent.change(screen.getByLabelText("sourceConnection"), {
      target: { value: alertmanagerConnection.id },
    });

    const lifecycleContract = screen.getByTestId("alertmanager-lifecycle-contract");
    expect(lifecycleContract).toHaveAttribute("role", "status");
    expect(lifecycleContract).toHaveTextContent("alertmanagerLifecycleContract");

    fireEvent.click(screen.getByRole("button", { name: "createRule" }));
    expect(createMutation.mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        name: ruleName,
        trigger_connection_id: alertmanagerConnection.id,
        trigger_kind: triggerKind,
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
    expect(
      within(table)
        .getAllByRole("columnheader")
        .slice(0, 6)
        .map(
          (header) =>
            header.querySelector("select")?.getAttribute("aria-label") ?? header.textContent,
        ),
    ).toEqual(["colRule", "colStatus", "colTrigger", "colResponse", "colNextRun", "colUpdated"]);
    const headers = within(table).getAllByRole("columnheader");
    expect(headers[4]).toHaveClass("whitespace-nowrap");
    expect(table.parentElement).toHaveClass("overflow-x-auto");
    expect(table.parentElement?.parentElement).not.toHaveClass("pt-6");
    expect(screen.getByRole("list", { name: "rulesList" }).parentElement).not.toHaveClass("pt-6");
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

  describe("projectRule", () => {
    it("projects rule capability labels and formatted dates", () => {
      const actions = [
        { name: "github_ci_failed", label: "CI failed", description: "CI failed", connection_service: "github", fields: [], service: "github", builtIn: false },
      ];
      const reactions = [
        { name: "create_incident", label: "Create Incident", description: "Create Incident", connection_service: "opswarden", fields: [], service: "opswarden", builtIn: true },
      ];

      const projected = projectRule(rule, actions, reactions, "en", "Disabled");

      expect(projected.rule).toBe(rule);
      expect(projected.triggerLabel).toBe("CI failed");
      expect(projected.reactionLabel).toBe("Create Incident");
      expect(projected.nextRunLabel).toBe("—");
      expect(projected.updatedAtLabel).toContain("2026");

      const disabledProjected = projectRule({ ...rule, enabled: false }, actions, reactions, "en", "Disabled");
      expect(disabledProjected.nextRunLabel).toBe("Disabled");
    });
  });
});
