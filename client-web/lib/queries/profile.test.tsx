import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import { profileQueryKey, useProfile, useUpdateLocale } from "./profile";

const mocks = vi.hoisted(() => ({
  setUser: vi.fn(),
}));

vi.mock("../api", () => ({
  apiFetch: vi.fn(),
}));

vi.mock("@/store/auth", () => ({
  useAuthStore: {
    getState: () => ({ setUser: mocks.setUser }),
  },
}));

const mockedApiFetch = vi.mocked(apiFetch);

function profile(locale: "en" | "fr") {
  return { id: "user-1", email: "user@example.com", locale };
}

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

afterEach(() => {
  vi.clearAllMocks();
});

describe("persisted profile locale", () => {
  it("loads the server profile including its locale", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(profile("en")));
    const { result } = renderHook(() => useProfile(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/me");
    expect(result.current.data?.locale).toBe("en");
  });

  it("persists fr and synchronizes both auth and query caches", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(profile("fr")));
    const { result } = renderHook(() => useUpdateLocale(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync("fr");
    });

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/me/locale", {
      method: "PUT",
      body: JSON.stringify({ locale: "fr" }),
    });
    expect(mocks.setUser).toHaveBeenCalledWith(profile("fr"));
    expect(queryClient.getQueryData(profileQueryKey)).toEqual(profile("fr"));
  });
});
