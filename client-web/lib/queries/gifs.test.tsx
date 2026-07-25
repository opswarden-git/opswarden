import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import { giphyEntryUrl, useGifSearch } from "./gifs";

vi.mock("../api", () => ({ apiFetch: vi.fn() }));
const mockedApiFetch = vi.mocked(apiFetch);

afterEach(() => vi.resetAllMocks());

describe("GIPHY timeline entries", () => {
  it("accepts only HTTPS giphy.com hosts", () => {
    expect(giphyEntryUrl("giphy:https://media.giphy.com/media/abc/giphy.gif")).toBe(
      "https://media.giphy.com/media/abc/giphy.gif",
    );
    expect(giphyEntryUrl("plain timeline note")).toBeNull();
    expect(giphyEntryUrl("giphy:http://media.giphy.com/unsafe.gif")).toBeNull();
    expect(giphyEntryUrl("giphy:https://evil.example/unsafe.gif")).toBeNull();
    expect(giphyEntryUrl("giphy:not a url")).toBeNull();
  });

  it("searches through the authenticated proxy with trimmed encoded input", async () => {
    const queryClient = createTestQueryClient();
    const gifs = [{ id: "gif-1", title: "Deploy", url: "https://giphy.com/deploy" }];
    mockedApiFetch.mockResolvedValueOnce(
      new Response(JSON.stringify(gifs), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );
    const { result } = renderHook(() => useGifSearch(" deploy now "), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/giphy/search?q=deploy%20now&limit=18");
    expect(result.current.data).toEqual(gifs);
  });

  it("does not query blank input and exposes stable server errors", async () => {
    const blank = renderHook(() => useGifSearch("   "), {
      wrapper: queryClientWrapper(createTestQueryClient()),
    });
    expect(blank.result.current.fetchStatus).toBe("idle");

    mockedApiFetch.mockResolvedValueOnce(
      new Response(JSON.stringify({ code: "giphy_unavailable" }), {
        status: 503,
        headers: { "Content-Type": "application/json" },
      }),
    );
    const failing = renderHook(() => useGifSearch("deploy"), {
      wrapper: queryClientWrapper(createTestQueryClient()),
    });
    await waitFor(() => expect(failing.result.current.error?.message).toBe("giphy_unavailable"));
  });
});
