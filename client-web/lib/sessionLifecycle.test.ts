import { QueryClient } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/store/auth";
import { useWsStore } from "./wsState";
import { endSession, establishSession, registerSessionQueryClient } from "./sessionLifecycle";

let unregisterQueryClient: (() => void) | undefined;

afterEach(async () => {
  unregisterQueryClient?.();
  unregisterQueryClient = undefined;
  await endSession();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe("session lifecycle", () => {
  it("purges account A server and realtime state before installing account B", async () => {
    const queryClient = new QueryClient();
    const replacementQueryClient = new QueryClient();
    unregisterQueryClient = registerSessionQueryClient(queryClient, () => replacementQueryClient);
    queryClient.setQueryData(["profile"], { id: "account-a" });
    queryClient.setQueryData(["private-messages"], [{ body: "account-a secret" }]);
    queryClient.getMutationCache().build(queryClient, {
      mutationKey: ["update-profile"],
      mutationFn: async () => undefined,
    });

    const oldSocketSend = vi.fn();
    const room = { kind: "direct", id: "account-a-peer" } as const;
    useWsStore.getState().setSendJson(oldSocketSend);
    useWsStore.getState().setRoomWatchers(room, ["account-a"]);
    useWsStore.getState().addRoomTypingUser(room, "account-a");
    useWsStore.getState().watchRoom(room);
    useWsStore.getState().setCursor("incident-a", "account-a", 0.2, 0.4);
    useWsStore.getState().setTeamOnline("team-a", ["account-a"]);
    useAuthStore.getState().setToken("token-a");
    useAuthStore.getState().setUser({ id: "account-a", locale: "en" });

    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          Response.json({ id: "account-b", email: "b@example.com", locale: "en" }),
        ),
    );
    await establishSession("token-b");

    expect(queryClient.getQueryCache().getAll()).toHaveLength(0);
    expect(queryClient.getMutationCache().getAll()).toHaveLength(0);
    queryClient.setQueryData(["late-account-a-result"], { secret: true });
    expect(replacementQueryClient.getQueryData(["late-account-a-result"])).toBeUndefined();
    expect(useWsStore.getState()).toMatchObject({
      watchersByRoom: {},
      typingByRoom: {},
      activeRooms: [],
      cursorsByIncident: {},
      onlineByTeam: {},
    });
    useWsStore.getState().sendJson({ type: "refresh_teams" });
    expect(oldSocketSend).toHaveBeenCalledTimes(1);
    expect(useAuthStore.getState()).toMatchObject({
      token: "token-b",
      user: { id: "account-b", email: "b@example.com", locale: "en" },
    });
  });

  it("prevents account A timers from mutating account B realtime state", async () => {
    vi.useFakeTimers();
    const room = { kind: "incident", id: "shared-incident" } as const;
    useWsStore.getState().addRoomTypingUser(room, "shared-user");
    useWsStore.getState().setCursor(room.id, "shared-user", 0.1, 0.1);
    await vi.advanceTimersByTimeAsync(1700);

    await endSession();
    useAuthStore.getState().setSession("token-b", { id: "account-b", locale: "en" });
    useWsStore.getState().addRoomTypingUser(room, "shared-user");
    useWsStore.getState().setCursor(room.id, "shared-user", 0.8, 0.8);
    await vi.advanceTimersByTimeAsync(1300);

    expect(useWsStore.getState().typingByRoom["incident:shared-incident"]).toEqual(["shared-user"]);
    expect(useWsStore.getState().cursorsByIncident[room.id]?.["shared-user"]).toMatchObject({
      x: 0.8,
      y: 0.8,
    });
  });

  it("clears identity state when the session ends", async () => {
    const queryClient = new QueryClient();
    unregisterQueryClient = registerSessionQueryClient(queryClient);
    queryClient.setQueryData(["teams"], [{ id: "team-a" }]);
    useAuthStore.getState().setToken("token-a");

    await endSession();

    expect(queryClient.getQueryData(["teams"])).toBeUndefined();
    expect(useAuthStore.getState().token).toBeNull();
  });

  it("keeps the current identity intact when a candidate profile cannot be loaded", async () => {
    useAuthStore.getState().setSession("token-a", { id: "account-a", locale: "en" });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 503 })));

    await expect(establishSession("token-b")).rejects.toThrow("profile_load_failed");

    expect(useAuthStore.getState()).toMatchObject({
      token: "token-a",
      user: { id: "account-a" },
    });
  });
});
