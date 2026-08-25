import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWsStore } from "@/lib/ws";
import { useAuthStore } from "@/store/auth";
import { IncidentActivity } from "./IncidentActivity";

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => {
    const translate = (key: string, values?: Record<string, unknown>) =>
      values ? `${key}:${Object.values(values).join(":")}` : key;
    translate.has = () => true;
    return translate;
  },
}));

const addEntry = { error: null, isPending: false, mutate: vi.fn() };
const editEntry = { error: null, isPending: false, mutate: vi.fn(), reset: vi.fn() };
const toggleReaction = { error: null, isPending: false, mutate: vi.fn() };

const activity = [
  {
    type: "system_event" as const,
    id: "event-created",
    kind: "created" as const,
    actor: null,
    subject: null,
    data: { status: "open", severity: "high" },
    created_at: "2026-07-25T10:00:00Z",
  },
  {
    type: "system_event" as const,
    id: "event-assigned",
    kind: "assigned" as const,
    actor: { user_id: "manager-1", email: "manager@example.com" },
    subject: { user_id: "user-1", email: "responder@example.com" },
    data: {},
    created_at: "2026-07-25T10:01:00Z",
  },
  {
    type: "system_event" as const,
    id: "event-severity",
    kind: "severity_changed" as const,
    actor: { user_id: "manager-1", email: "manager@example.com" },
    subject: null,
    data: { from: "high", to: "critical" },
    created_at: "2026-07-25T10:02:00Z",
  },
  {
    type: "system_event" as const,
    id: "event-status",
    kind: "status_changed" as const,
    actor: { user_id: "user-1", email: "responder@example.com" },
    subject: null,
    data: { from: "open", to: "acknowledged" },
    created_at: "2026-07-25T10:03:00Z",
  },
  {
    type: "system_event" as const,
    id: "event-status-repeat",
    kind: "status_changed" as const,
    actor: { user_id: "user-1", email: "responder@example.com" },
    subject: null,
    data: { from: "open", to: "acknowledged" },
    created_at: "2026-07-25T10:03:30Z",
  },
  {
    type: "human_note" as const,
    entry_id: "entry-1",
    author: { user_id: "user-1", email: "responder@example.com" },
    content: "Investigating the primary database",
    created_at: "2026-07-25T10:04:00Z",
    edited_at: null,
    attachments: [],
    reactions: [{ emoji: "👍", count: 1, reacted: true }],
  },
  {
    type: "human_note" as const,
    entry_id: "entry-gif",
    author: { user_id: "user-2", email: "peer@example.com" },
    content: "giphy:https://media.giphy.com/media/abc/giphy.gif",
    created_at: "2026-07-25T10:05:00Z",
    edited_at: "2026-07-25T10:06:00Z",
    attachments: [],
    reactions: [],
  },
];

vi.mock("@/lib/queries/incidents", () => ({
  useIncidentActivity: () => ({
    data: activity,
    error: null,
    features: ["send_text", "attach_files", "collaborative_cursors"],
    isLoading: false,
  }),
  useAvailableReactions: () => ({ data: ["👍", "✅"] }),
  useAddTimelineEntry: () => addEntry,
  useEditTimelineEntry: () => editEntry,
  useToggleTimelineReaction: () => toggleReaction,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAuthStore.getState().logout();
  useWsStore.setState({ sendJson: () => {}, typingByRoom: {}, cursorsByIncident: {} });
});

describe("IncidentActivity", () => {
  it("renders system history, editable notes, GIFs and reactions", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "responder@example.com", locale: "en" });
    render(<IncidentActivity canCompose incidentId="incident-1" people={{}} />);

    expect(screen.getByText(/activityCreated/)).toBeInTheDocument();
    expect(screen.getByText(/activityAssigned/)).toBeInTheDocument();
    expect(screen.getByText(/activitySeverityChanged/)).toBeInTheDocument();
    expect(screen.getAllByText(/activityStatusChanged/)).toHaveLength(2);
    expect(screen.getAllByText("activityTransitionFrom")).toHaveLength(3);
    expect(screen.getAllByText("activityTransitionTo")).toHaveLength(3);
    expect(screen.queryByText(/activityEventCount/)).not.toBeInTheDocument();
    expect(screen.getAllByText("statusOpen")).toHaveLength(3);
    expect(screen.getAllByText("statusAcknowledged")).toHaveLength(2);
    expect(screen.getAllByText("severityHigh")).toHaveLength(2);
    expect(screen.getByText("severityCritical")).toBeInTheDocument();
    expect(screen.getAllByText("statusOpen")[0].parentElement).toHaveClass("bg-status-neutral");
    expect(screen.getAllByText("statusAcknowledged")[0].parentElement).toHaveClass(
      "bg-status-info",
    );
    expect(screen.getAllByText(/activityStatusChanged/)[0].parentElement).toHaveAttribute(
      "title",
      expect.stringContaining("2026"),
    );
    expect(document.querySelector(".lucide-refresh-ccw")).not.toBeInTheDocument();
    expect(document.querySelector(".lucide-arrow-right")).not.toBeInTheDocument();
    expect(screen.getByText("Investigating the primary database")).toBeInTheDocument();
    expect(
      screen.getByText("Investigating the primary database").closest("[data-note-owner]"),
    ).toHaveAttribute("data-note-owner", "current");
    expect(screen.getByText("Investigating the primary database").parentElement).toHaveClass(
      "bg-gold",
    );
    expect(
      screen.getByRole("img", { name: "gifAlt" }).closest("[data-note-owner]"),
    ).toHaveAttribute("data-note-owner", "peer");
    expect(screen.queryByRole("heading", { name: "activity" })).not.toBeInTheDocument();
    expect(screen.getByRole("img", { name: "gifAlt" })).toHaveAttribute(
      "src",
      "https://media.giphy.com/media/abc/giphy.gif",
    );

    fireEvent.click(screen.getAllByRole("button", { name: "👍 (1)" })[0]);
    expect(toggleReaction.mutate).toHaveBeenCalledWith({
      incidentId: "incident-1",
      entryId: "entry-1",
      emoji: "👍",
    });
  });

  it("edits an owned human note", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "responder@example.com", locale: "en" });
    render(<IncidentActivity canCompose incidentId="incident-1" people={{}} />);

    fireEvent.click(screen.getByRole("button", { name: "edit" }));
    const editor = screen.getByRole("textbox", { name: "editNote" });
    fireEvent.change(editor, { target: { value: "Database failover started" } });
    fireEvent.click(screen.getByRole("button", { name: "save" }));

    expect(editEntry.reset).toHaveBeenCalledOnce();
    expect(editEntry.mutate).toHaveBeenCalledWith(
      {
        incidentId: "incident-1",
        entryId: "entry-1",
        content: "Database failover started",
      },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });

  it("sends a trimmed note and emits throttled typing presence", async () => {
    const sendJson = vi.fn();
    useWsStore.setState({ sendJson });
    render(
      <IncidentActivity
        canCompose
        incidentId="incident-1"
        people={{ "user-2": "peer@example.com" }}
      />,
    );

    const composer = screen.getByRole("textbox", { name: "addNote" });
    expect(composer.closest("[data-conversation-composer]")).not.toHaveClass("border-t");
    fireEvent.change(composer, { target: { value: "  Mitigation deployed  " } });
    fireEvent.click(screen.getByRole("button", { name: "send" }));

    expect(sendJson).toHaveBeenCalledWith({ type: "status_typing", incident_id: "incident-1" });
    await waitFor(() =>
      expect(addEntry.mutate).toHaveBeenCalledWith(
        { incidentId: "incident-1", content: "Mitigation deployed", attachments: [] },
        expect.objectContaining({ onSuccess: expect.any(Function) }),
      ),
    );
  });

  it("shares normalized pointer movement and renders peer cursors", () => {
    const sendJson = vi.fn();
    useWsStore.setState({
      sendJson,
      cursorsByIncident: {
        "incident-1": {
          "user-2": { userId: "user-2", x: 0.25, y: 0.5, updatedAt: Date.now() },
        },
      },
    });
    render(
      <IncidentActivity
        canCompose
        incidentId="incident-1"
        people={{ "user-2": "peer@example.com" }}
      />,
    );

    const room = screen.getByRole("region", { name: "warRoomConversation" });
    vi.spyOn(room, "getBoundingClientRect").mockReturnValue({
      bottom: 600,
      height: 500,
      left: 100,
      right: 900,
      top: 100,
      width: 800,
      x: 100,
      y: 100,
      toJSON: () => ({}),
    });
    fireEvent.pointerMove(room, { clientX: 300, clientY: 350, pointerType: "mouse" });

    expect(sendJson).toHaveBeenCalledWith({
      type: "cursor",
      incident_id: "incident-1",
      x: 0.25,
      y: 0.5,
    });
    const peerCursor = document.querySelector('[data-collaborator-cursor="user-2"]');
    expect(peerCursor).toHaveTextContent("peer");
    expect(peerCursor?.querySelector("path")).toHaveAttribute("fill", "var(--gold)");
    expect(peerCursor?.querySelector("span")).toHaveClass("bg-gold", "text-gold-ink");
  });
});
