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
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "responder" as const,
  created_at: "2026-07-25T09:00:00Z",
  member_count: 3,
  active_incident_count: 2,
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

let loading = false;
let failing = false;
vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({
    data: [team],
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

afterEach(() => {
  cleanup();
  loading = false;
  failing = false;
  useAuthStore.getState().logout();
});

describe("TeamOverviewPage", () => {
  it("projects operational counts and ranked cross-resource attention", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "operator@example.com", locale: "en" });
    render(<TeamOverviewPage teamId="team-1" />);

    expect(screen.getByRole("heading", { name: "Operations" })).toBeInTheDocument();
    expect(screen.getByText("operationalSummary")).toBeInTheDocument();
    expect(screen.getAllByText("Database outage").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Production deploy")).toHaveLength(2);
    expect(screen.getAllByText("blockedReleases")).toHaveLength(2);
  });

  it("renders explicit loading and error boundaries", () => {
    loading = true;
    const view = render(<TeamOverviewPage teamId="team-1" />);
    expect(screen.getByLabelText("loadingOverview")).toBeInTheDocument();
    view.unmount();

    loading = false;
    failing = true;
    render(<TeamOverviewPage teamId="team-1" />);
    expect(screen.getByText("overviewUnavailable")).toBeInTheDocument();
  });
});
