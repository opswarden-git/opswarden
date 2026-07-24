import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import {
  useConfigureTeamConnection,
  useRefreshServiceOAuth,
  useStartServiceOAuth,
} from "./automations";

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

describe("GitHub service OAuth", () => {
  it("starts the authorization flow through the authenticated Team endpoint", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({ authorization_url: "https://github.com/login/oauth/authorize?safe" }),
    );
    const { result } = renderHook(() => useStartServiceOAuth("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await expect(
        result.current.mutateAsync({ locale: "fr", service: "github" }),
      ).resolves.toEqual({
        authorization_url: "https://github.com/login/oauth/authorize?safe",
      });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith(
      "/api/teams/team-1/service-connections/by-service/github/oauth/start?locale=fr",
      { method: "POST" },
    );
  });

  it("configures any catalog service through the generic endpoint", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ id: "connection-1" }));
    const { result } = renderHook(() => useConfigureTeamConnection("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        service: "future-service",
        payload: { api_key: "encrypted-server-side" },
      });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith(
      "/api/teams/team-1/service-connections/by-service/future-service",
      {
        method: "PUT",
        body: JSON.stringify({ api_key: "encrypted-server-side" }),
      },
    );
  });

  it("refreshes OAuth without accepting or returning token material", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch.mockResolvedValueOnce(
      jsonResponse({
        id: "connection-1",
        team_id: "team-1",
        service: "github",
        secret_configured: true,
        token_configured: false,
        oauth_configured: true,
        oauth_refresh_configured: true,
        created_at: "2026-07-24T00:00:00Z",
        updated_at: "2026-07-24T00:00:00Z",
        verified_at: "2026-07-24T00:00:00Z",
        last_delivery_at: null,
        last_error_code: null,
        webhook_path: "/webhooks/github/connection-1",
      }),
    );
    const { result } = renderHook(() => useRefreshServiceOAuth("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      const connection = await result.current.mutateAsync("connection-1");
      expect(connection.oauth_configured).toBe(true);
      expect(connection.oauth_refresh_configured).toBe(true);
      expect(connection).not.toHaveProperty("access_token");
      expect(connection).not.toHaveProperty("refresh_token");
    });

    expect(mockedApiFetch).toHaveBeenCalledWith(
      "/api/teams/team-1/service-connections/connection-1/oauth/refresh",
      { method: "POST" },
    );
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["team-automation-connections", "team-1"],
    });
  });
});
