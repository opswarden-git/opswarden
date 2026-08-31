import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import { TeamOverviewPage } from "./TeamOverviewPage";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values ? `${key}:${Object.values(values).join(":")}` : key,
}));
vi.mock("@/i18n/routing", () => ({
  usePathname: () => "/teams/team-1/overview",
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager" as const,
  created_at: "2026-07-25T09:00:00Z",
  member_count: 3,
  active_incident_count: 1,
  active_release_count: 1,
  blocked_release_count: 1,
};
const queue = {
  items: [
    {
      id: "incident-1",
      team_id: "team-1",
      title: "Database outage",
      description: "",
      status: "escalated" as const,
      severity: "critical" as const,
      assignee: { user_id: "user-1", email: "operator@example.com" },
      created_at: "2026-07-25T10:00:00Z",
      created_by: null,
      updated_at: "2026-07-25T10:05:00Z",
    },
  ],
  counts: { all: 1, open: 0, acknowledged: 0, escalated: 1, resolved: 0 },
};
const releases = [
  {
    release_id: "release-1",
    team_id: "team-1",
    title: "Production deploy",
    state: "blocked" as const,
    progress: { completed: 1, total: 2 },
    next_step: { position: 1, name: "Deploy" },
    blockers: [
      {
        incident_id: "incident-1",
        title: "Database outage",
        status: "escalated" as const,
        severity: "critical" as const,
      },
    ],
    linked_incident_ids: ["incident-1"],
    created_at: "2026-07-25T09:00:00Z",
    updated_at: "2026-07-25T10:05:00Z",
  },
];

const rules = [
  {
    id: "rule-1",
    name: "Open production incident",
  },
];
const runs = [
  {
    id: "run-1",
    delivery_id: "delivery-1",
    rule_id: "rule-1",
    status: "succeeded",
    incident_id: "incident-1",
    error_code: null,
    started_at: "2026-07-25T10:04:00Z",
    finished_at: "2026-07-25T10:04:01Z",
  },
];

let loading = false;
let failing = false;
let teamRole: "manager" | "responder" | "observer" = "manager";
vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({
    data: [{ ...team, role: teamRole }],
    isLoading: loading,
    error: failing ? new Error("failed") : null,
  }),
}));
vi.mock("@/lib/queries/incidents", () => ({
  useIncidentQueue: () => ({
    data: queue,
    isLoading: loading,
    error: failing ? new Error("failed") : null,
  }),
}));
vi.mock("@/lib/queries/releases", () => ({
  useReleases: () => ({
    data: releases,
    isLoading: loading,
    error: failing ? new Error("failed") : null,
  }),
}));
vi.mock("@/lib/queries/automations", () => ({
  useAutomationRules: () => ({
    data: rules,
    isLoading: loading,
    error: failing ? new Error("failed") : null,
  }),
  useAutomationRuns: () => ({
    data: runs,
    isLoading: loading,
    isFetching: false,
    error: failing ? new Error("failed") : null,
    refetch: vi.fn(),
  }),
}));

afterEach(() => {
  cleanup();
  loading = false;
  failing = false;
  teamRole = "manager";
  useAuthStore.getState().logout();
});

describe("TeamOverviewPage", () => {
  it("shows Incidents, Releases and Runs together", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "operator@example.com", locale: "en" });
    render(<TeamOverviewPage teamId="team-1" />);

    // The page names the page. The Team is named once, by the sidebar, so this
    // header no longer repeats it above every screen.
    expect(screen.queryByRole("heading", { name: "Operations" })).toBeNull();
    expect(screen.queryByText("needsAttention")).toBeNull();
    expect(screen.getByText("Database outage")).toBeInTheDocument();
    expect(screen.getByText("Production deploy")).toBeInTheDocument();
    expect(screen.getByText("Open production incident")).toBeInTheDocument();
    expect(screen.getByRole("grid", { name: /calendar\.label/ })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "overviewViewsLabel" })).toBeNull();
    expect(
      screen.getByRole("link", {
        name: "overviewViews.seeAll: overviewViews.incidents",
      }),
    ).toHaveAttribute("href", "/teams/team-1/incidents");
    expect(
      screen.getByRole("link", {
        name: "overviewViews.seeAll: overviewViews.releases",
      }),
    ).toHaveAttribute("href", "/teams/team-1/releases");
    expect(
      screen.getByRole("link", { name: "overviewViews.seeAll: overviewViews.runs" }),
    ).toHaveAttribute("href", "/teams/team-1/runs");
  });

  it("keeps automation runs out of roles that cannot manage them", () => {
    teamRole = "responder";
    render(<TeamOverviewPage teamId="team-1" />);
    expect(screen.getByText("Database outage")).toBeInTheDocument();
    expect(screen.getByText("Production deploy")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "overviewViews.runs" })).toBeNull();
  });

  it("renders explicit loading and error boundaries", () => {
    loading = true;
    const view = render(<TeamOverviewPage teamId="team-1" />);
    const skeleton = screen.getByLabelText("loadingOverview");
    expect(skeleton.querySelectorAll('[data-skeleton-calendar-day="true"]')).toHaveLength(42);
    expect(skeleton.querySelectorAll('[data-skeleton-region="overview-summary"]')).toHaveLength(3);
    view.unmount();

    loading = false;
    failing = true;
    render(<TeamOverviewPage teamId="team-1" />);
    expect(screen.getByText("overviewUnavailable")).toBeInTheDocument();
  });
});
