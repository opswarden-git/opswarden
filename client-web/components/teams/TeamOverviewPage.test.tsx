import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import { TeamOverviewPage } from "./TeamOverviewPage";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values ? `${key}:${Object.values(values).join(":")}` : key,
}));
vi.mock("next/navigation", () => ({
  useSearchParams: () => new URLSearchParams(),
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
  it("shows one cross-resource inbox, each item once, with counted facets", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "operator@example.com", locale: "en" });
    render(<TeamOverviewPage teamId="team-1" />);

    // The page names the page. The Team is named once, by the sidebar, so this
    // header no longer repeats it above every screen.
    expect(screen.queryByRole("heading", { name: "Operations" })).toBeNull();
    expect(screen.getByText("needsAttention")).toBeInTheDocument();
    expect(screen.getAllByText("Database outage").length).toBeGreaterThan(0);

    // The defect this replaced: a blocked Release appeared twice on the same
    // screen -- once in the inbox, once in a side panel that repeated it. The
    // previous version of this test asserted the duplication as expected.
    expect(screen.getAllByText("Production deploy")).toHaveLength(1);

    // Facets are URL-backed views onto that one queue, not links elsewhere.
    const facets = screen.getByRole("navigation", { name: "attentionFacetsLabel" });
    expect(within(facets).getByRole("link", { name: /facets\.all/ })).toBeInTheDocument();
    expect(within(facets).getByRole("link", { name: /facets\.blocked/ })).toBeInTheDocument();
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
