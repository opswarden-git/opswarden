import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import {
  useAddTimelineEntry,
  useAssignIncident,
  useAvailableReactions,
  useCreateIncident,
  useDeleteIncident,
  useEditTimelineEntry,
  useIncident,
  useIncidentActivity,
  useIncidentQueue,
  useIncidents,
  useToggleTimelineReaction,
  useUpdateIncidentStatus,
} from "./incidents";

vi.mock("../api", () => ({
  apiFetch: vi.fn(),
}));

const mockedApiFetch = vi.mocked(apiFetch);

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function incidentResponse(overrides: Record<string, unknown> = {}) {
  return {
    incident_id: "incident-1",
    team_id: "team-1",
    title: "Database latency",
    description: "Elevated latency",
    status: "open",
    severity: "high",
    assignee_id: null,
    created_at: "2026-07-25T10:00:00Z",
    created_by: "user-1",
    updated_at: "2026-07-25T10:00:00Z",
    actions: {
      can_assign: true,
      can_delete: true,
      can_write_timeline: true,
      transitions: ["acknowledged"],
    },
    ...overrides,
  };
}

afterEach(() => {
  vi.resetAllMocks();
});

describe("available incident reactions", () => {
  it("loads the canonical catalog from the server endpoint", async () => {
    const queryClient = createTestQueryClient();
    const reactions = ["👍", "👀", "✅", "🚨", "❤️", "🎉"];
    mockedApiFetch.mockResolvedValueOnce(
      new Response(JSON.stringify({ reactions }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const { result } = renderHook(() => useAvailableReactions(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledOnce();
    expect(mockedApiFetch).toHaveBeenCalledWith("/reactions/available");
    expect(result.current.data).toEqual(reactions);
  });
});

describe("incident read models", () => {
  it("loads and normalizes a filtered incident queue", async () => {
    const queryClient = createTestQueryClient();
    const listItem = {
      ...incidentResponse(),
      assignee: { user_id: "user-2", email: "responder@example.com" },
    };
    delete (listItem as { assignee_id?: unknown }).assignee_id;
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({
        items: [listItem],
        counts: { all: 4, open: 1, acknowledged: 1, escalated: 1, resolved: 1 },
      }),
    );

    const { result } = renderHook(
      () =>
        useIncidentQueue("team-1", {
          status: "open",
          severity: "high",
          assignee: "user-2",
          query: " database ",
          sort: "severity",
        }),
      { wrapper: queryClientWrapper(queryClient) },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(mockedApiFetch).toHaveBeenCalledWith(
      "/api/incidents?team_id=team-1&status=open&severity=high&assignee=user-2&q=database&sort=severity",
    );
    expect(result.current.data?.items[0]).toMatchObject({
      id: "incident-1",
      assignee: { user_id: "user-2" },
    });
    expect(result.current.data?.counts.all).toBe(4);
  });

  it("keeps queues disabled without a team and exposes the unfiltered picker", async () => {
    const disabledClient = createTestQueryClient();
    const disabled = renderHook(() => useIncidentQueue(undefined, {}), {
      wrapper: queryClientWrapper(disabledClient),
    });
    expect(disabled.result.current.fetchStatus).toBe("idle");

    const queryClient = createTestQueryClient();
    const listItem = { ...incidentResponse(), assignee: null };
    delete (listItem as { assignee_id?: unknown }).assignee_id;
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({
        items: [listItem],
        counts: { all: 1, open: 1, acknowledged: 0, escalated: 0, resolved: 0 },
      }),
    );
    const picker = renderHook(() => useIncidents("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    await waitFor(() => expect(picker.result.current.isSuccess).toBe(true));
    expect(picker.result.current.data).toHaveLength(1);
  });

  it("loads and normalizes incident detail and activity", async () => {
    const queryClient = createTestQueryClient();
    const activity = [{ type: "human_note", entry_id: "entry-1", content: "Investigating" }];
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse(incidentResponse({ assignee_id: "user-2" })))
      .mockResolvedValueOnce(
        jsonResponse({ items: activity, features: ["send_text", "system_events"] }),
      );

    const detail = renderHook(() => useIncident("incident-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const timeline = renderHook(() => useIncidentActivity("incident-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(detail.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(timeline.result.current.isSuccess).toBe(true));
    expect(detail.result.current.data).toMatchObject({
      id: "incident-1",
      assignee: "user-2",
      actions: { canAssign: true, transitions: ["acknowledged"] },
    });
    expect(timeline.result.current.data).toEqual(activity);
    expect(timeline.result.current.features).toEqual(["send_text", "system_events"]);
  });

  it("walks the war room backwards with the cursor the server hands back", async () => {
    const queryClient = createTestQueryClient();
    const newest = { type: "human_note", entry_id: "entry-2", content: "Escalating" };
    const older = { type: "human_note", entry_id: "entry-1", content: "Investigating" };
    mockedApiFetch
      .mockResolvedValueOnce(
        jsonResponse({
          items: [newest],
          next_cursor: { created_at: "2026-08-24T10:00:00Z", id: "entry-2" },
          features: ["send_text", "paginated_history"],
        }),
      )
      .mockResolvedValueOnce(jsonResponse({ items: [older], next_cursor: null }));

    const timeline = renderHook(() => useIncidentActivity("incident-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    await waitFor(() => expect(timeline.result.current.isSuccess).toBe(true));
    expect(timeline.result.current.hasNextPage).toBe(true);

    await act(async () => {
      await timeline.result.current.fetchNextPage();
    });

    // The cursor travels as query parameters, and both pages are held at once.
    const [url] = mockedApiFetch.mock.calls[1];
    expect(url).toContain("before_created_at=2026-08-24T10%3A00%3A00Z");
    expect(url).toContain("before_id=entry-2");
    await waitFor(() => expect(timeline.result.current.data).toEqual([newest, older]));
    expect(timeline.result.current.hasNextPage).toBe(false);
  });

  it("reports the first page's features and never a later page's", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({ items: [], next_cursor: null, features: ["send_text"] }),
    );

    const timeline = renderHook(() => useIncidentActivity("incident-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(timeline.result.current.isSuccess).toBe(true));
    expect(timeline.result.current.features).toEqual(["send_text"]);
  });
});

describe("incident mutations", () => {
  it("creates an incident with a default empty description", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ id: "incident-new" }, 201));
    const { result } = renderHook(() => useCreateIncident(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        team_id: "team-1",
        title: "API unavailable",
        severity: "critical",
      });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/incidents", {
      method: "POST",
      body: JSON.stringify({
        team_id: "team-1",
        title: "API unavailable",
        description: "",
        severity: "critical",
      }),
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incidents"] });
  });

  it("adds, edits and reacts to timeline entries", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch
      .mockResolvedValueOnce(new Response("entry-1"))
      .mockResolvedValueOnce(jsonResponse({ entry_id: "entry-1", content: "Updated" }))
      .mockResolvedValueOnce(jsonResponse({ reacted: true }));
    const add = renderHook(() => useAddTimelineEntry(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const edit = renderHook(() => useEditTimelineEntry(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const react = renderHook(() => useToggleTimelineReaction(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await add.result.current.mutateAsync({ incidentId: "incident-1", content: "Investigating" });
      await edit.result.current.mutateAsync({
        incidentId: "incident-1",
        entryId: "entry-1",
        content: "Updated",
      });
      await react.result.current.mutateAsync({
        incidentId: "incident-1",
        entryId: "entry-1",
        emoji: "✅",
      });
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/incidents/incident-1/timeline", {
      method: "POST",
      body: JSON.stringify({ content: "Investigating", attachments: [] }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(
      2,
      "/api/incidents/incident-1/timeline/entry-1",
      {
        method: "PUT",
        body: JSON.stringify({ content: "Updated" }),
      },
    );
    expect(mockedApiFetch).toHaveBeenNthCalledWith(
      3,
      "/api/incidents/incident-1/timeline/entry-1/reactions",
      { method: "POST", body: JSON.stringify({ emoji: "✅" }) },
    );
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["activity", "incident-1"] });
  });

  it("updates status and assignment and refreshes all incident projections", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ status: "acknowledged" }))
      .mockResolvedValueOnce(jsonResponse({ assignee_id: "user-2" }));
    const status = renderHook(() => useUpdateIncidentStatus(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const assign = renderHook(() => useAssignIncident(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await status.result.current.mutateAsync({ incidentId: "incident-1", status: "acknowledged" });
      await assign.result.current.mutateAsync({ incidentId: "incident-1", assigneeId: "user-2" });
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/incidents/incident-1/status", {
      method: "PUT",
      body: JSON.stringify({ status: "acknowledged" }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/incidents/incident-1/assign", {
      method: "PUT",
      body: JSON.stringify({ assignee_id: "user-2" }),
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incident", "incident-1"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incidents"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["activity", "incident-1"] });
  });

  it("deletes detail cache and refreshes the queue", async () => {
    const queryClient = createTestQueryClient();
    const remove = vi.spyOn(queryClient, "removeQueries");
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch.mockResolvedValueOnce(new Response(null, { status: 204 }));
    const hook = renderHook(() => useDeleteIncident(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => hook.result.current.mutateAsync("incident-1"));

    expect(remove).toHaveBeenCalledWith({ queryKey: ["incident", "incident-1"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incidents"] });
  });

  it("surfaces stable backend error codes and safe fallbacks", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ code: "invalid_transition" }, 409))
      .mockResolvedValueOnce(new Response("not-json", { status: 500 }))
      .mockResolvedValueOnce(jsonResponse({ code: "invalid_timeline_attachment" }, 400));
    const status = renderHook(() => useUpdateIncidentStatus(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const edit = renderHook(() => useEditTimelineEntry(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const add = renderHook(() => useAddTimelineEntry(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await expect(
      status.result.current.mutateAsync({ incidentId: "incident-1", status: "resolved" }),
    ).rejects.toThrow("invalid_transition");
    await expect(
      edit.result.current.mutateAsync({
        incidentId: "incident-1",
        entryId: "entry-1",
        content: "Updated",
      }),
    ).rejects.toThrow("edit_timeline_entry_failed");
    await expect(
      add.result.current.mutateAsync({ incidentId: "incident-1", content: "" }),
    ).rejects.toThrow("invalid_timeline_attachment");
  });
});
