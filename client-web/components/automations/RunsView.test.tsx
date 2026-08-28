import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AutomationRule, AutomationRun } from "@/lib/queries/automations";
import { RunsView } from "./RunsView";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: (namespace: string) => {
    const translate = (key: string) => (namespace === "errors" ? `error:${key}` : key);
    translate.has = () => true;
    return translate;
  },
}));

vi.mock("@/i18n/routing", () => ({
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const rule: AutomationRule = {
  id: "rule-1",
  team_id: "team-1",
  name: "Failed CI",
  trigger_connection_id: "connection-1",
  trigger_kind: "github_ci_failed",
  trigger_config: {},
  reaction_kind: "create_incident",
  reaction_connection_id: null,
  reaction_config: {},
  enabled: true,
  created_by: null,
  created_at: "2026-07-25T10:00:00Z",
  updated_at: "2026-07-25T10:00:00Z",
  next_run_at: null,
};

const run: AutomationRun = {
  id: "run-12345678",
  delivery_id: "delivery-1",
  rule_id: rule.id,
  status: "succeeded",
  incident_id: null,
  error_code: null,
  started_at: "2026-07-25T10:00:00Z",
  finished_at: "2026-07-25T10:00:01Z",
};

afterEach(cleanup);

describe("RunsView", () => {
  it("uses the shared table hierarchy and exposes the run as the row identity", () => {
    render(<RunsView rules={[rule]} runs={[run]} teamId="team-1" />);

    const table = screen.getByRole("table", { name: "runsList" });
    expect(
      within(table)
        .getAllByRole("columnheader")
        .map((header) => header.textContent),
    ).toEqual(["colRun", "colStatus", "colRule", "colResult", "colStarted", "colDuration"]);
    expect(within(table).getByRole("rowheader", { name: "run-1234" })).toHaveAttribute(
      "scope",
      "row",
    );
  });

  it("localizes status filters and stable error codes", () => {
    const failedRun = {
      ...run,
      id: "run-failed-1234",
      status: "failed",
      error_code: "reaction_timeout",
    };
    render(
      <RunsView
        rules={[rule]}
        runs={[run, failedRun]}
        teamId="team-1"
        showControls
        statusFilter="failed"
      />,
    );

    const statusFilter = screen.getByRole("combobox", { name: "colStatus" });
    expect(
      within(statusFilter)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["allStatuses", "runStatusFailed", "runStatusSucceeded"]);
    expect(screen.getByText("error:reaction_timeout")).toBeInTheDocument();
    expect(screen.queryByText("reaction_timeout")).not.toBeInTheDocument();
  });
});
