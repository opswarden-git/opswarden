import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReleaseListItem } from "@/lib/queries/releases";
import { ReleasesPage } from "./ReleasesPage";

const push = vi.fn();
const replace = vi.fn();
let params = new URLSearchParams();

vi.mock("@/i18n/routing", () => ({
  usePathname: () => "/teams/team-1/releases",
  useRouter: () => ({ push, replace }),
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));
vi.mock("next/navigation", () => ({ useSearchParams: () => params }));
vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string, values?: Record<string, unknown>) =>
    values ? `${key}:${Object.values(values).join(":")}` : key,
}));
vi.mock("@/components/releases/CreateReleaseDialog", () => ({
  CreateReleaseDialog: () => <button>createRelease</button>,
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager" as const,
  created_at: "",
  member_count: 2,
  active_incident_count: 1,
  active_release_count: 1,
  blocked_release_count: 1,
};
const activeRelease: ReleaseListItem = {
  release_id: "release-1",
  team_id: "team-1",
  title: "Production deployment",
  state: "in_progress",
  progress: { completed: 1, total: 3 },
  next_step: { position: 1, name: "Deploy" },
  blockers: [],
  linked_incident_ids: [],
  created_at: "2026-07-25T09:00:00Z",
  updated_at: "2026-07-25T10:00:00Z",
};
const blockedRelease: ReleaseListItem = {
  ...activeRelease,
  release_id: "release-2",
  title: "Emergency rollout",
  state: "blocked",
  blockers: [
    {
      incident_id: "incident-1",
      title: "Database outage",
      status: "escalated",
      severity: "critical",
    },
  ],
};

let teamsData: (typeof team)[] = [team];
let releasesData: ReleaseListItem[] = [activeRelease, blockedRelease];
let loading = false;
let queryError: Error | null = null;

vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({ data: teamsData, isLoading: loading }),
}));
vi.mock("@/lib/queries/releases", () => ({
  useReleases: () => ({ data: releasesData, isLoading: loading, error: queryError }),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  params = new URLSearchParams();
  teamsData = [team];
  releasesData = [activeRelease, blockedRelease];
  loading = false;
  queryError = null;
});

describe("ReleasesPage", () => {
  it("renders the active release in desktop and mobile projections", () => {
    render(<ReleasesPage teamId="team-1" />);

    expect(screen.queryByRole("heading", { name: "title" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "createRelease" })).toBeInTheDocument();
    expect(screen.getAllByText("Production deployment")).toHaveLength(2);
    expect(screen.getAllByText("Emergency rollout")).toHaveLength(2);
    expect(screen.getAllByRole("progressbar")).toHaveLength(4);
    const table = screen.getByRole("table", { name: "tableLabel" });
    expect(
      within(table)
        .getAllByRole("columnheader")
        .map(
          (header) =>
            header.querySelector("select")?.getAttribute("aria-label") ?? header.textContent,
        ),
    ).toEqual(["colRelease", "colStatus", "colProgress", "colNextStep", "colBlockers", "colAge"]);
    fireEvent.change(screen.getByRole("combobox", { name: "colStatus" }), {
      target: { value: "blocked" },
    });
    expect(push).toHaveBeenCalledWith("/teams/team-1/releases?view=blocked");
  });

  it("preserves the selected view and redirects legacy detail URLs", () => {
    params = new URLSearchParams("view=blocked&release=release-2");
    render(<ReleasesPage teamId="team-1" />);

    expect(screen.getAllByText("Emergency rollout")).toHaveLength(2);
    expect(screen.getAllByText("Database outage")).toHaveLength(2);
    expect(replace).toHaveBeenCalledWith("/teams/team-1/releases/release-2?view=blocked");
  });

  it("searches releases by title and preserves collection state in the URL", async () => {
    params = new URLSearchParams("view=all&q=emergency");
    render(<ReleasesPage teamId="team-1" />);

    expect(screen.getAllByText("Emergency rollout")).toHaveLength(2);
    expect(screen.queryByText("Production deployment")).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("searchbox", { name: "searchLabel" }), {
      target: { value: "production" },
    });
    await waitFor(() =>
      expect(replace).toHaveBeenCalledWith("/teams/team-1/releases?view=all&q=production"),
    );
  });

  it("distinguishes filtered-empty, no-team, error, and loading states", () => {
    params = new URLSearchParams("view=completed");
    const filtered = render(<ReleasesPage teamId="team-1" />);
    expect(screen.getByText("noMatchingReleases")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "clearFilters" }));
    expect(push).toHaveBeenCalledWith("/teams/team-1/releases?view=all");
    filtered.unmount();

    teamsData = [];
    releasesData = [];
    const withoutTeam = render(<ReleasesPage teamId="team-1" />);
    expect(screen.getByText("noTeamsYet")).toBeInTheDocument();
    withoutTeam.unmount();

    teamsData = [team];
    queryError = new Error("failed");
    const failed = render(<ReleasesPage teamId="team-1" />);
    expect(screen.getByText("failedToLoad")).toBeInTheDocument();
    failed.unmount();

    queryError = null;
    loading = true;
    render(<ReleasesPage teamId="team-1" />);
    const skeleton = screen.getByTestId("release-skeleton-desktop");
    expect(within(skeleton).getAllByRole("columnheader")).toHaveLength(6);
  });
});
