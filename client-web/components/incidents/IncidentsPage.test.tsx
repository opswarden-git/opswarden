import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { IncidentsPage } from "./IncidentsPage";

const push = vi.fn();
const replace = vi.fn();
vi.mock("@/i18n/routing", () => ({
  usePathname: () => "/teams/team-1/incidents",
  useRouter: () => ({ push, replace }),
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));
vi.mock("next/navigation", () => ({
  useSearchParams: () => new URLSearchParams("view=all&severity=critical&sort=severity"),
}));
vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values ? `${key}:${Object.values(values).join(":")}` : key,
}));
vi.mock("@/components/incidents/CreateIncidentDialog", () => ({
  CreateIncidentDialog: () => <button>createIncident</button>,
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager" as const,
  created_at: "",
  member_count: 2,
  active_incident_count: 1,
  active_release_count: 0,
  blocked_release_count: 0,
};
const item = {
  id: "incident-1",
  team_id: "team-1",
  title: "Database outage",
  description: "Primary unavailable",
  status: "open" as const,
  severity: "critical" as const,
  assignee: { user_id: "responder-1", email: "responder@example.com" },
  created_at: "2026-07-25T10:00:00Z",
  created_by: null,
  updated_at: "2026-07-25T10:00:00Z",
};
let teamsData: (typeof team)[] = [team];
let queueItems: (typeof item)[] = [item];
let queryError: Error | null = null;
let loading = false;

vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({ data: teamsData, isLoading: loading }),
  useTeamMembers: () => ({
    data: [
      {
        user_id: "manager-1",
        email: "manager@example.com",
        role: "manager",
        can_be_assigned_incident: true,
      },
      {
        user_id: "responder-1",
        email: "responder@example.com",
        role: "responder",
        can_be_assigned_incident: true,
      },
    ],
  }),
}));
vi.mock("@/lib/queries/incidents", () => ({
  useIncidentQueue: () => ({
    data: {
      items: queueItems,
      counts: {
        all: queueItems.length ? 1 : 0,
        open: queueItems.length ? 1 : 0,
        acknowledged: 0,
        escalated: 0,
        resolved: 0,
      },
    },
    isLoading: loading,
    error: queryError,
  }),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  teamsData = [team];
  queueItems = [item];
  queryError = null;
  loading = false;
});

describe("IncidentsPage", () => {
  it("renders queue filters and updates URL-backed state", () => {
    render(<IncidentsPage teamId="team-1" />);
    expect(screen.queryByRole("heading", { name: "title" })).not.toBeInTheDocument();
    expect(screen.getAllByText("Database outage")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "createIncident" })).toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox", { name: "colSeverity" }), {
      target: { value: "high" },
    });
    expect(push).toHaveBeenCalledWith(
      "/teams/team-1/incidents?view=all&severity=high&sort=severity",
    );
    fireEvent.change(screen.getByRole("combobox", { name: "colAssignee" }), {
      target: { value: "responder-1" },
    });
    expect(push).toHaveBeenCalledWith(expect.stringContaining("assignee=responder-1"));
  });

  it("separates no-team, filtered-empty, loading and error boundaries", () => {
    teamsData = [];
    queueItems = [];
    const view = render(<IncidentsPage teamId="team-1" />);
    expect(screen.getByText("noTeamsYet")).toBeInTheDocument();
    view.unmount();

    teamsData = [team];
    queryError = new Error("failed");
    const errorView = render(<IncidentsPage teamId="team-1" />);
    expect(screen.getByText("failedToLoad")).toBeInTheDocument();
    errorView.unmount();

    queryError = null;
    loading = true;
    render(<IncidentsPage teamId="team-1" />);
    expect(screen.getByTestId("incident-skeleton-desktop")).toBeInTheDocument();
  });
});
