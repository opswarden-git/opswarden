import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import { useAvailableReactions } from "./incidents";

vi.mock("../api", () => ({
  apiFetch: vi.fn(),
}));

const mockedApiFetch = vi.mocked(apiFetch);

afterEach(() => {
  vi.clearAllMocks();
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
