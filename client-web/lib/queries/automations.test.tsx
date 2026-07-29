import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import {
  useAutomationCatalog,
  useAutomationRules,
  useAutomationRuns,
  useConfigureTeamConnection,
  useCreateAutomationRule,
  useDeleteAutomationRule,
  useDeleteTeamConnection,
  useRefreshServiceOAuth,
  useStartServiceOAuth,
  useTeamConnections,
  useTestTeamConnection,
  useUpdateAutomationRule,
} from "./automations";

vi.mock("next-intl", () => ({ useLocale: () => "en" }));

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
  vi.resetAllMocks();
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

  it("configures Generic Webhook through the catalog-driven endpoint", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch.mockResolvedValueOnce(jsonResponse({ id: "connection-1" }));
    const { result } = renderHook(() => useConfigureTeamConnection("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await result.current.mutateAsync({
        service: "generic",
        payload: { webhook_signing_secret: "encrypted-server-side" },
      });
    });

    expect(mockedApiFetch).toHaveBeenCalledWith(
      "/api/teams/team-1/service-connections/by-service/generic",
      {
        method: "PUT",
        body: JSON.stringify({ webhook_signing_secret: "encrypted-server-side" }),
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

describe("automation queries", () => {
  it("loads the localized catalog, connections, rules and bounded runs", async () => {
    const queryClient = createTestQueryClient();
    const catalog = [{ name: "github", actions: [], reactions: [], connection: null }];
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ server: { services: catalog } }))
      .mockResolvedValueOnce(jsonResponse([{ id: "connection-1" }]))
      .mockResolvedValueOnce(jsonResponse([{ id: "rule-1" }]))
      .mockResolvedValueOnce(jsonResponse([{ id: "run-1" }]));

    const catalogHook = renderHook(() => useAutomationCatalog(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const connections = renderHook(() => useTeamConnections("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const rules = renderHook(() => useAutomationRules("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const runs = renderHook(() => useAutomationRuns("team-1", true, 25), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(catalogHook.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(connections.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(rules.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(runs.result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/about.json?locale=en");
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/service-connections");
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/automation-rules");
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/automation-runs?limit=25");
    expect(catalogHook.result.current.data).toEqual(catalog);
  });

  it("keeps every optional automation query idle when disabled", () => {
    const queryClient = createTestQueryClient();
    const wrapper = queryClientWrapper(queryClient);
    const catalog = renderHook(() => useAutomationCatalog(false), { wrapper });
    const connections = renderHook(() => useTeamConnections("", true), { wrapper });
    const rules = renderHook(() => useAutomationRules("team-1", false), { wrapper });
    const runs = renderHook(() => useAutomationRuns("team-1", false), { wrapper });

    expect(catalog.result.current.fetchStatus).toBe("idle");
    expect(connections.result.current.fetchStatus).toBe("idle");
    expect(rules.result.current.fetchStatus).toBe("idle");
    expect(runs.result.current.fetchStatus).toBe("idle");
    expect(mockedApiFetch).not.toHaveBeenCalled();
  });
});

describe("automation mutations", () => {
  it("tests and deletes a connection, invalidating dependent projections", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const testConnection = renderHook(() => useTestTeamConnection("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const removeConnection = renderHook(() => useDeleteTeamConnection("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await testConnection.result.current.mutateAsync("connection-1");
      await removeConnection.result.current.mutateAsync("connection-1");
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(
      1,
      "/api/teams/team-1/service-connections/connection-1/test",
      { method: "POST" },
    );
    expect(mockedApiFetch).toHaveBeenNthCalledWith(
      2,
      "/api/teams/team-1/service-connections/connection-1",
      { method: "DELETE" },
    );
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["team-automation-connections", "team-1"],
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-automation-rules", "team-1"] });
  });

  it("creates, updates and deletes rules through their canonical endpoints", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    const definition = {
      name: "CI failure",
      trigger_connection_id: "connection-1",
      trigger_kind: "github_ci_failed",
      trigger_config: {},
      reaction_kind: "create_incident",
      reaction_connection_id: null,
      reaction_config: { severity: "high" },
    };
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ id: "rule-1", ...definition }, 201))
      .mockResolvedValueOnce(jsonResponse({ id: "rule-1", ...definition, enabled: false }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const create = renderHook(() => useCreateAutomationRule("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const update = renderHook(() => useUpdateAutomationRule("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const remove = renderHook(() => useDeleteAutomationRule("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await create.result.current.mutateAsync(definition);
      await update.result.current.mutateAsync({ ruleId: "rule-1", enabled: false });
      await remove.result.current.mutateAsync("rule-1");
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams/team-1/automation-rules", {
      method: "POST",
      body: JSON.stringify(definition),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams/team-1/automation-rules/rule-1", {
      method: "PATCH",
      body: JSON.stringify({ enabled: false }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(3, "/api/teams/team-1/automation-rules/rule-1", {
      method: "DELETE",
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-automation-rules", "team-1"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-automation-runs", "team-1"] });
  });

  it("uses backend codes when available and stable fallbacks otherwise", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ code: "connection_unreachable" }, 422))
      .mockResolvedValueOnce(new Response("not-json", { status: 500 }));
    const testConnection = renderHook(() => useTestTeamConnection("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const removeRule = renderHook(() => useDeleteAutomationRule("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await expect(testConnection.result.current.mutateAsync("connection-1")).rejects.toThrow(
      "connection_unreachable",
    );
    await expect(removeRule.result.current.mutateAsync("rule-1")).rejects.toThrow(
      "automation_rule_delete_failed",
    );
  });
});
