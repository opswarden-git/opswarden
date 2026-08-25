import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Release } from "@/lib/queries/releases";
import { ReleaseDetailPage } from "./ReleaseDetailPage";

const replace = vi.fn();
const cancelRelease = {
  error: null as Error | null,
  isPending: false,
  mutate: vi.fn(),
  reset: vi.fn(),
};
let params = new URLSearchParams("view=blocked");

vi.mock("next/navigation", () => ({ useSearchParams: () => params }));
vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));
vi.mock("@/i18n/routing", () => ({
  useRouter: () => ({ replace }),
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));
vi.mock("./ReleaseDetail", () => ({
  ReleaseDetail: ({ release, onCancel }: { release: Release; onCancel?: () => void }) => (
    <div>
      detail:{release.title}
      {onCancel ? <button onClick={onCancel}>cancelRelease</button> : null}
    </div>
  ),
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager" as const,
  created_at: "",
  member_count: 2,
  active_incident_count: 0,
  active_release_count: 1,
  blocked_release_count: 0,
};
const baseRelease: Release = {
  release_id: "release-1",
  team_id: "team-1",
  title: "Production deployment",
  state: "in_progress",
  steps: [
    {
      position: 0,
      name: "Build",
      validated: true,
      validated_by: "manager-1",
      validated_at: "2026-07-25T09:30:00Z",
    },
    {
      position: 1,
      name: "Deploy",
      validated: false,
      validated_by: null,
      validated_at: null,
    },
  ],
  linked_incident_ids: [],
  created_at: "2026-07-25T09:00:00Z",
  updated_at: "2026-07-25T10:00:00Z",
};

let teamsData: (typeof team)[] = [team];
let releaseData: Release | undefined = baseRelease;
let loading = false;
let queryError: Error | null = null;

vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({ data: teamsData, isLoading: loading }),
}));
vi.mock("@/lib/queries/releases", () => ({
  useRelease: () => ({ data: releaseData, isLoading: loading, error: queryError }),
  useCancelRelease: () => cancelRelease,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  params = new URLSearchParams("view=blocked");
  teamsData = [team];
  releaseData = baseRelease;
  loading = false;
  queryError = null;
  cancelRelease.error = null;
  cancelRelease.isPending = false;
});

describe("ReleaseDetailPage", () => {
  it("renders metadata and confirms cancellation", () => {
    render(<ReleaseDetailPage teamId="team-1" releaseId="release-1" />);

    expect(screen.getByRole("heading", { name: "Production deployment" })).toBeInTheDocument();
    expect(screen.getByText(/createdOn:/)).toBeInTheDocument();
    expect(screen.getByText("detail:Production deployment")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "cancelRelease" }));
    expect(cancelRelease.reset).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "cancelRelease" }));
    expect(cancelRelease.mutate).toHaveBeenCalledWith(
      { releaseId: "release-1", teamId: "team-1" },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });

  it("redirects a release that belongs to another team and hides terminal actions", () => {
    releaseData = { ...baseRelease, team_id: "team-2", state: "completed" };
    render(<ReleaseDetailPage teamId="team-1" releaseId="release-1" />);

    expect(replace).toHaveBeenCalledWith("/teams/team-2/releases/release-1?view=blocked");
    expect(screen.queryByRole("button", { name: "cancelRelease" })).not.toBeInTheDocument();
  });

  it("renders loading and error boundaries", () => {
    loading = true;
    const pending = render(<ReleaseDetailPage teamId="team-1" releaseId="release-1" />);
    expect(screen.getByTestId("release-detail-skeleton")).toBeInTheDocument();
    pending.unmount();

    loading = false;
    releaseData = undefined;
    queryError = new Error("failed");
    render(<ReleaseDetailPage teamId="team-1" releaseId="release-1" />);
    expect(screen.getByText("failedToLoadDetail")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "backToReleases" })).toHaveAttribute(
      "href",
      "/teams/team-1/releases?view=blocked",
    );
  });
});
