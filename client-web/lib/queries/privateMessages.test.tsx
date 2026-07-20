import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import { usePrivateMessages, useSendPrivateMessage } from "./privateMessages";

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

afterEach(() => {
  vi.clearAllMocks();
});

describe("private message mutations", () => {
  it("does not load a conversation before its dialog opens", () => {
    const queryClient = createTestQueryClient();

    const { result } = renderHook(() => usePrivateMessages("peer-1", false), {
      wrapper: queryClientWrapper(queryClient),
    });

    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedApiFetch).not.toHaveBeenCalled();
  });

  it("loads a peer conversation newest-first from the backend", async () => {
    const queryClient = createTestQueryClient();
    const messages = [
      {
        id: "pm-new",
        sender_id: "peer-1",
        recipient_id: "me",
        content: "newest",
        created_at: "2026-06-26T00:00:01Z",
      },
      {
        id: "pm-old",
        sender_id: "me",
        recipient_id: "peer-1",
        content: "oldest",
        created_at: "2026-06-26T00:00:00Z",
      },
    ];
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ messages }));

    const { result } = renderHook(() => usePrivateMessages("peer-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/private-messages?peer_id=peer-1");
    expect(result.current.data).toEqual(messages);
  });

  it("invalidates exactly the recipient conversation after sending", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({
        id: "pm-1",
        sender_id: "me",
        recipient_id: "peer-1",
        content: "hello",
        created_at: "2026-06-26T00:00:00Z",
      }),
    );

    const { result } = renderHook(() => useSendPrivateMessage(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({ recipientId: "peer-1", content: "hello" });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/private-messages", {
      method: "POST",
      body: JSON.stringify({ recipient_id: "peer-1", content: "hello" }),
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["private-messages", "peer-1"] });
  });

  it("surfaces backend error codes when send fails", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ code: "no_shared_team" }, 403));

    const { result } = renderHook(() => useSendPrivateMessage(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await expect(
      result.current.mutateAsync({ recipientId: "peer-2", content: "hello" }),
    ).rejects.toThrow("no_shared_team");
  });
});
