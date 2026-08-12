import { cleanup, fireEvent, render, screen } from "@testing-library/react";
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
    data: {},
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
    reactions: [{ emoji: "👍", count: 1, reacted: true }],
  },
  {
    type: "human_note" as const,
    entry_id: "entry-gif",
    author: { user_id: "user-2", email: "peer@example.com" },
    content: "giphy:https://media.giphy.com/media/abc/giphy.gif",
    created_at: "2026-07-25T10:05:00Z",
    edited_at: "2026-07-25T10:06:00Z",
    reactions: [],
  },
];

vi.mock("@/lib/queries/incidents", () => ({
  useIncidentActivity: () => ({ data: activity, error: null, isLoading: false }),
  useAvailableReactions: () => ({ data: ["👍", "✅"] }),
  useAddTimelineEntry: () => addEntry,
  useEditTimelineEntry: () => editEntry,
  useToggleTimelineReaction: () => toggleReaction,
}));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAuthStore.getState().logout();
  useWsStore.setState({ sendJson: () => {}, typingByIncident: {} });
});

describe("IncidentActivity", () => {
  it("renders system history, editable notes, GIFs and reactions", () => {
    useAuthStore.getState().setUser({ id: "user-1", email: "responder@example.com", locale: "en" });
    render(<IncidentActivity canCompose incidentId="incident-1" people={{}} />);

    expect(screen.getByText(/activityCreated/)).toBeInTheDocument();
    expect(screen.getByText(/activityAssigned/)).toBeInTheDocument();
    expect(screen.getByText(/activitySeverityChanged/)).toBeInTheDocument();
    expect(screen.getAllByText(/activityStatusChanged/)).toHaveLength(1);
    expect(screen.getByText("activityEventCount:2")).toBeInTheDocument();
    const eventTitle = screen
      .getByText(/activityStatusChanged/)
      .parentElement?.getAttribute("title");
    expect(eventTitle?.split("\n")).toHaveLength(2);
    expect(eventTitle).toContain("2026");
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

  it("sends a trimmed note and emits throttled typing presence", () => {
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
    expect(addEntry.mutate).toHaveBeenCalledWith(
      { incidentId: "incident-1", content: "Mitigation deployed" },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });
});
