import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWsStore } from "@/lib/ws";
import { IncidentDetailPage } from "./IncidentDetailPage";

const push = vi.fn();
const replace = vi.fn();
vi.mock("@/i18n/routing", () => ({
  useRouter: () => ({ push, replace }),
  Link: ({ children, href, ...props }: React.ComponentProps<"a">) => (
    <a href={String(href)} {...props}>
      {children}
    </a>
  ),
}));

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

const incident = {
  id: "incident-12345678",
  team_id: "team-1",
  title: "Database outage",
  description: "Primary database unavailable",
  status: "open" as const,
  severity: "critical" as const,
  assignee: "responder-1",
  created_at: "2026-07-25T10:00:00Z",
  created_by: "manager-1",
  updated_at: "2026-07-25T10:05:00Z",
};

let incidentQuery: { data: typeof incident | undefined; isLoading: boolean; error: Error | null } =
  {
    data: incident,
    isLoading: false,
    error: null,
  };
const updateStatus = { error: null, isPending: false, mutate: vi.fn() };
const deleteIncident = {
  error: null,
  isPending: false,
  mutate: vi.fn(),
  reset: vi.fn(),
};
const assignIncident = { error: null, isPending: false, mutate: vi.fn() };
const addEntry = { error: null, isPending: false, mutate: vi.fn() };
const editEntry = { error: null, isPending: false, mutate: vi.fn(), reset: vi.fn() };
const toggleReaction = { error: null, isPending: false, mutate: vi.fn() };

vi.mock("@/lib/queries/incidents", () => ({
  useIncident: () => incidentQuery,
  useIncidents: () => ({ data: [incident] }),
  useUpdateIncidentStatus: () => updateStatus,
  useDeleteIncident: () => deleteIncident,
  useAssignIncident: () => assignIncident,
  useIncidentActivity: () => ({ data: [], error: null, isLoading: false }),
  useAvailableReactions: () => ({ data: ["👍"] }),
  useAddTimelineEntry: () => addEntry,
  useEditTimelineEntry: () => editEntry,
  useToggleTimelineReaction: () => toggleReaction,
}));

const team = {
  team_id: "team-1",
  name: "Operations",
  role: "manager" as const,
  created_at: "2026-07-25T09:00:00Z",
  member_count: 2,
  active_incident_count: 1,
  active_release_count: 1,
  blocked_release_count: 1,
};
const members = [
  { user_id: "manager-1", email: "manager@example.com", role: "manager" as const, joined_at: "" },
  {
    user_id: "responder-1",
    email: "responder@example.com",
    role: "responder" as const,
    joined_at: "",
  },
];

vi.mock("@/lib/queries/teams", () => ({
  useTeams: () => ({ data: [team] }),
  useTeamMembers: () => ({ data: members }),
}));

vi.mock("@/lib/queries/releases", () => ({
  useReleases: () => ({
    data: [
      {
        release_id: "release-1",
        title: "Production deploy",
        state: "blocked",
        linked_incident_ids: [incident.id],
      },
    ],
    error: null,
    isLoading: false,
  }),
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  incidentQuery = { data: incident, isLoading: false, error: null };
  useWsStore.setState({
    watchersByIncident: { [incident.id]: ["manager-1", "responder-1", "unknown"] },
    activeWatches: [],
    sendJson: () => {},
  });
});

describe("IncidentDetailPage", () => {
  it("renders the incident workspace and executes the primary transition", () => {
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);

    expect(screen.getByRole("heading", { name: "Database outage" })).toBeInTheDocument();
    expect(screen.getAllByText("responder@example.com").length).toBeGreaterThan(1);
    expect(screen.getByText("Production deploy")).toBeInTheDocument();
    expect(screen.getAllByText("manager@example.com").length).toBeGreaterThan(0);
    expect(screen.queryByText("teamLabel")).not.toBeInTheDocument();
    expect(screen.queryByText("createdAt")).not.toBeInTheDocument();
    expect(screen.queryByText("updatedAt")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /acknowledge/ }));
    expect(updateStatus.mutate).toHaveBeenCalledWith({
      incidentId: incident.id,
      status: "acknowledged",
    });
  });

  it("changes assignee and opens the mobile context sheet", () => {
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    const assignee = screen.getByRole("combobox", { name: "changeAssignee" });
    fireEvent.change(assignee, { target: { value: "manager-1" } });
    fireEvent.click(screen.getByRole("button", { name: "assign" }));
    expect(assignIncident.mutate).toHaveBeenCalledWith({
      incidentId: incident.id,
      assigneeId: "manager-1",
    });

    fireEvent.click(screen.getByRole("button", { name: "details" }));
    expect(screen.getByRole("dialog", { name: "incidentContext" })).toBeInTheDocument();
  });

  it("can fully retract and restore both desktop war-room rails", () => {
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);

    fireEvent.click(screen.getByRole("button", { name: "collapseRooms" }));
    fireEvent.click(screen.getByRole("button", { name: "collapseContext" }));
    expect(document.querySelector('[data-rooms-rail-open="false"]')).toBeInTheDocument();
    expect(document.querySelector('[data-context-rail-open="false"]')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "expandRooms" }));
    fireEvent.click(screen.getByRole("button", { name: "expandContext" }));
    expect(document.querySelector('[data-rooms-rail-open="true"]')).toBeInTheDocument();
    expect(document.querySelector('[data-context-rail-open="true"]')).toBeInTheDocument();
  });

  it("renders deterministic loading and error states", () => {
    incidentQuery = { data: undefined, isLoading: true, error: null };
    const view = render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    expect(document.querySelector(".animate-pulse")).toBeInTheDocument();

    view.unmount();
    incidentQuery = { data: undefined, isLoading: false, error: new Error("load_failed") };
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    expect(screen.getByText("failedToLoadIncident")).toBeInTheDocument();
  });
});
