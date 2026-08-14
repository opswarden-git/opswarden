import { QueryClient } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import {
  createDesktopNotificationGate,
  desktopNotificationForEvent,
  dispatchDesktopNotification,
  handleWsContractEvent,
  useWsStore,
  webSocketUrl,
} from "./ws";

function queryClientWithInvalidationSpy() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return {
    queryClient,
    invalidate: vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue(),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  useAuthStore.getState().logout();
  useWsStore.setState({
    watchersByIncident: {},
    cursorsByIncident: {},
    activeWatches: [],
    sendJson: () => {},
  });
});

describe("WebSocket deployment URL", () => {
  it("uses an explicit build-time override when configured", () => {
    vi.stubEnv("NEXT_PUBLIC_WS_URL", "wss://api.example.test/ws");

    expect(webSocketUrl()).toBe("wss://api.example.test/ws");
  });

  it("falls back to the browser origin and /ws ingress route", () => {
    vi.stubEnv("NEXT_PUBLIC_WS_URL", "");

    const url = new URL(webSocketUrl()!);
    expect(url.host).toBe(window.location.host);
    expect(url.pathname).toBe("/ws");
    expect(url.protocol).toBe(window.location.protocol === "https:" ? "wss:" : "ws:");
  });
});

describe("WebSocket contract consumers", () => {
  it("expires collaborator pointers without letting an older timer remove a newer position", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    const store = useWsStore.getState();
    store.setCursor("incident-1", "peer", 0.2, 0.3);
    vi.advanceTimersByTime(1_000);
    useWsStore.getState().setCursor("incident-1", "peer", 0.7, 0.8);
    vi.advanceTimersByTime(900);

    expect(useWsStore.getState().cursorsByIncident["incident-1"].peer).toMatchObject({
      x: 0.7,
      y: 0.8,
    });
    vi.advanceTimersByTime(900);
    expect(useWsStore.getState().cursorsByIncident["incident-1"].peer).toBeUndefined();
    vi.useRealTimers();
  });

  it("indexes generic presence by its incident resource id", () => {
    const { queryClient } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "presence_update",
        resource_id: "incident-1",
        resource_type: "incident",
        watchers: ["user-1", "user-2"],
      },
      queryClient,
    );

    expect(useWsStore.getState().watchersByIncident["incident-1"]).toEqual(["user-1", "user-2"]);
  });

  it("refreshes a peer roster after a kick", () => {
    useAuthStore.getState().setUser({ id: "me", locale: "en" });
    const { queryClient, invalidate } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "member_kicked",
        team_id: "team-1",
        member: "peer",
        by: "manager",
      },
      queryClient,
    );

    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["teams"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incidents"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incident"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-members", "team-1"] });
  });

  it("refreshes its socket team scope when the current user is banned", () => {
    useAuthStore.getState().setUser({ id: "me", locale: "en" });
    const sendJson = vi.fn();
    useWsStore.setState({ sendJson });
    const { queryClient, invalidate } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "member_banned",
        team_id: "team-1",
        member: "me",
        until: null,
        by: "manager",
      },
      queryClient,
    );

    expect(sendJson).toHaveBeenCalledWith({ type: "refresh_teams" });
    expect(invalidate).not.toHaveBeenCalledWith({
      queryKey: ["team-members", "team-1"],
    });
  });

  it("invalidates only the other participant's private conversation", () => {
    useAuthStore.getState().setUser({ id: "me", locale: "en" });
    const { queryClient, invalidate } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "private_message_received",
        from: "peer",
        to: "me",
        content: "ping",
        at: 1_784_901_600,
      },
      queryClient,
    );

    expect(invalidate).toHaveBeenCalledTimes(1);
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: ["private-messages", "peer"],
    });
  });

  it("consumes the canonical successful-rule result fields", () => {
    const { queryClient, invalidate } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "rule_triggered",
        service: "opswarden",
        rule_name: "CI failure to incident",
        result: "incident_created",
        incident_id: "incident-1",
      },
      queryClient,
    );

    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["incidents"] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-automation-runs"] });
  });

  it("consumes the canonical failed-rule error code", () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    const { queryClient, invalidate } = queryClientWithInvalidationSpy();

    handleWsContractEvent(
      {
        type: "rule_failed",
        service: "http",
        rule_name: "Notify responder",
        error: "reaction_http_5xx",
      },
      queryClient,
    );

    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-automation-runs"] });
    expect(log).toHaveBeenCalledWith(
      "[Automation] Rule failed for http: Notify responder - reaction_http_5xx",
    );
  });
});

describe("desktop notification policy", () => {
  const translate: Parameters<typeof desktopNotificationForEvent>[2] = (key, values) =>
    values ? `${key}:${Object.values(values).join(":")}` : key;

  it("dispatches all three required notifications while the main window is hidden", () => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "hidden",
    });
    const notify = vi.fn();
    const gate = createDesktopNotificationGate();

    expect(
      dispatchDesktopNotification(
        {
          type: "incident_assigned",
          incident_id: "incident-assigned",
          assigned_to: "me",
          by: "manager",
        },
        "me",
        translate,
        gate,
        notify,
      ),
    ).toBe(true);
    expect(
      dispatchDesktopNotification(
        {
          type: "incident_created",
          incident_id: "incident-critical",
          severity: "critical",
        },
        "me",
        translate,
        gate,
        notify,
      ),
    ).toBe(true);
    expect(
      dispatchDesktopNotification(
        {
          type: "release_state_changed",
          release_id: "release-blocked",
          new_state: "blocked",
        },
        "me",
        translate,
        gate,
        notify,
      ),
    ).toBe(true);

    expect(document.visibilityState).toBe("hidden");
    expect(notify).toHaveBeenCalledTimes(3);
    expect(notify).toHaveBeenNthCalledWith(
      1,
      "incidentAssignedTitle",
      "incidentReference:incident",
    );
    expect(notify).toHaveBeenNthCalledWith(
      2,
      "incidentCriticalTitle",
      "incidentReference:incident",
    );
    expect(notify).toHaveBeenNthCalledWith(3, "releaseBlockedTitle", "releaseBlockedBody:release-");
  });

  it("does not notify a non-critical direct creation", () => {
    expect(
      desktopNotificationForEvent(
        { type: "incident_created", incident_id: "incident-high", severity: "high" },
        "me",
        translate,
      ),
    ).toBeNull();
  });

  it("deduplicates the same frame and a replayed frame across a reconnect window", () => {
    const gate = createDesktopNotificationGate();
    const event = {
      type: "release_state_changed" as const,
      release_id: "release-1",
      new_state: "blocked",
    };
    const fingerprint = "release-blocked:release-1";

    expect(gate(event, fingerprint, 1_000)).toBe(true);
    expect(gate(event, fingerprint, 1_001)).toBe(false);
    expect(gate({ ...event }, fingerprint, 1_002)).toBe(false);
    expect(gate({ ...event }, fingerprint, 31_002)).toBe(true);
  });
});
