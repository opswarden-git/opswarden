import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Incident } from "@/lib/queries/incidents";
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

const incident: Incident = {
  id: "incident-12345678",
  team_id: "team-1",
  title: "Database outage",
  description: "Primary database unavailable",
  status: "open",
  severity: "critical",
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
  useIncidentActivity: () => ({
    data: [],
    error: null,
    features: ["send_text", "attach_files", "collaborative_cursors"],
    isLoading: false,
  }),
  useMarkIncidentRead: () => ({ mutate: vi.fn() }),
  useAvailableReactions: () => ({ data: ["👍"] }),
  useAddTimelineEntry: () => addEntry,
  useEditTimelineEntry: () => editEntry,
  useToggleTimelineReaction: () => toggleReaction,
}));

let teamRole: "manager" | "responder" | "observer" = "manager";
const team = {
  team_id: "team-1",
  name: "Operations",
  get role() {
    return teamRole;
  },
  created_at: "2026-07-25T09:00:00Z",
  member_count: 2,
  active_incident_count: 1,
  active_release_count: 1,
  blocked_release_count: 1,
};
const members = [
  {
    user_id: "manager-1",
    email: "manager@example.com",
    role: "manager" as const,
    can_be_assigned_incident: true,
    joined_at: "",
  },
  {
    user_id: "responder-1",
    email: "responder@example.com",
    role: "responder" as const,
    can_be_assigned_incident: true,
    joined_at: "",
  },
];

vi.mock("@/lib/queries/privateMessages", () => ({
  useUnreadPrivateMessages: () => ({ data: { unread_peer_ids: [] } }),
}));

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
  teamRole = "manager";
  useWsStore.setState({
    watchersByRoom: {
      [`incident:${incident.id}`]: ["manager-1", "responder-1", "unknown"],
    },
    activeRooms: [],
    sendJson: () => {},
  });
});

describe("IncidentDetailPage", () => {
  it("renders the incident workspace and executes the primary transition", () => {
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);

    expect(screen.getByRole("heading", { name: "Database outage" })).toBeInTheDocument();
    const context = document.querySelector('[data-war-room-context="true"]') as HTMLElement;
    fireEvent.click(within(context).getByRole("button", { name: /colAssignee/ }));
    expect(within(context).getByText("responder@example.com")).toBeInTheDocument();
    // Le titre apparaît aussi en tête de groupe dans la nav : on vise le panneau.
    fireEvent.click(within(context).getByRole("button", { name: "linkedReleases" }));
    expect(within(context).getByText("Production deploy")).toBeInTheDocument();
    expect(screen.getAllByText("manager@example.com").length).toBeGreaterThan(0);
    expect(screen.queryByText("teamLabel")).not.toBeInTheDocument();
    expect(screen.queryByText("createdAt")).not.toBeInTheDocument();
    expect(screen.queryByText("updatedAt")).not.toBeInTheDocument();
    fireEvent.click(within(context).getByRole("button", { name: /moreActions/ }));
    fireEvent.click(screen.getByRole("button", { name: /acknowledge/ }));
    expect(updateStatus.mutate).toHaveBeenCalledWith({
      incidentId: incident.id,
      status: "acknowledged",
    });
  });

  it("changes assignee and opens the mobile context sheet", () => {
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    fireEvent.click(screen.getByRole("button", { name: /colAssignee/ }));
    const assignee = screen.getByRole("combobox", { name: "changeAssignee" });
    const assign = screen.getByRole("button", { name: "assign" });
    expect(assign).toHaveClass("text-muted", "hover:text-st-res", "hover:bg-transparent");
    fireEvent.change(assignee, { target: { value: "manager-1" } });
    fireEvent.click(assign);
    expect(assignIncident.mutate).toHaveBeenCalledWith({
      incidentId: incident.id,
      assigneeId: "manager-1",
    });

    fireEvent.click(screen.getByRole("button", { name: "incidentContext" }));
    expect(screen.getByRole("dialog", { name: "incidentContext" })).toBeInTheDocument();
  });

  it("signals acknowledgement and assignment independently", () => {
    const first = render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    const context = within(document.querySelector('[data-war-room-context="true"]') as HTMLElement);
    expect(context.getByRole("button", { name: "details" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(context.getByRole("button", { name: /moreActions/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(context.getByRole("button", { name: /colAssignee/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(context.getByRole("button", { name: "linkedReleases" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(context.getByRole("button", { name: "members" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(screen.getByLabelText("actionRequired")).toBeInTheDocument();
    expect(screen.queryByLabelText("assigneeRequired")).not.toBeInTheDocument();

    first.unmount();
    incidentQuery = {
      data: { ...incident, status: "acknowledged", assignee: null },
      isLoading: false,
      error: null,
    };
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    expect(screen.queryByLabelText("actionRequired")).not.toBeInTheDocument();
    expect(screen.getByLabelText("assigneeRequired")).toBeInTheDocument();
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

  it("does not render an empty Actions section for an Observer", () => {
    teamRole = "observer";
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);

    const context = within(document.querySelector('[data-war-room-context="true"]') as HTMLElement);
    expect(context.queryByRole("button", { name: /moreActions/ })).not.toBeInTheDocument();
    expect(context.getByRole("button", { name: /colAssignee/ })).toBeInTheDocument();
    expect(context.getByRole("button", { name: "members" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("renders deterministic loading and error states", () => {
    incidentQuery = { data: undefined, isLoading: true, error: null };
    const view = render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    expect(screen.getByTestId("conversation-room-skeleton")).toBeInTheDocument();
    expect(screen.getByTestId("incident-context-skeleton")).toBeInTheDocument();

    view.unmount();
    incidentQuery = { data: undefined, isLoading: false, error: new Error("load_failed") };
    render(<IncidentDetailPage incidentId={incident.id} teamId="team-1" />);
    expect(screen.getByText("failedToLoadIncident")).toBeInTheDocument();
  });
});
