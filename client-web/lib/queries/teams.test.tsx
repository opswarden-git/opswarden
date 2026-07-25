import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient, queryClientWrapper } from "../../test/reactQuery";
import { apiFetch } from "../api";
import { useWsStore } from "../ws";
import {
  useBanMember,
  useCreateTeam,
  useDeleteTeam,
  useInvitationCode,
  useJoinTeam,
  useKickMember,
  useLeaveTeam,
  useSetMemberRole,
  useTeamBans,
  useTeamMembers,
  useTeams,
  useTransferManager,
  useUnbanMember,
} from "./teams";

vi.mock("../api", () => ({ apiFetch: vi.fn() }));

const mockedApiFetch = vi.mocked(apiFetch);

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

afterEach(() => {
  vi.resetAllMocks();
  useWsStore.setState({ sendJson: () => {} });
});

describe("team queries", () => {
  it("loads teams and keeps member queries idle without a team", async () => {
    const queryClient = createTestQueryClient();
    const teams = [{ team_id: "team-1", name: "Operations", role: "manager" }];
    mockedApiFetch.mockResolvedValueOnce(jsonResponse(teams));

    const teamsHook = renderHook(() => useTeams(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const membersHook = renderHook(() => useTeamMembers(undefined), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(teamsHook.result.current.isSuccess).toBe(true));
    expect(teamsHook.result.current.data).toEqual(teams);
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams");
    expect(membersHook.result.current.fetchStatus).toBe("idle");
  });

  it("loads members, invitation code and bans for a selected team", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse([{ user_id: "user-1", role: "responder" }]))
      .mockResolvedValueOnce(jsonResponse({ invitation_code: "invite-1" }))
      .mockResolvedValueOnce(jsonResponse([{ kind: "permanent", active: true }]))
      .mockResolvedValueOnce(jsonResponse([]));

    const members = renderHook(() => useTeamMembers("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const invitation = renderHook(() => useInvitationCode("team-1", true), {
      wrapper: queryClientWrapper(queryClient),
    });
    const bans = renderHook(() => useTeamBans("team-1", true), {
      wrapper: queryClientWrapper(queryClient),
    });

    await waitFor(() => expect(members.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(invitation.result.current.isSuccess).toBe(true));
    await waitFor(() => expect(bans.result.current.isSuccess).toBe(true));

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/members");
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/invitation");
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/bans");
    expect(invitation.result.current.data).toEqual({ invitation_code: "invite-1" });

    const disabled = renderHook(() => useInvitationCode(undefined, false), {
      wrapper: queryClientWrapper(createTestQueryClient()),
    });
    expect(disabled.result.current.fetchStatus).toBe("idle");
  });
});

describe("team membership mutations", () => {
  it("creates and joins teams, refreshes the list and resyncs the socket scope", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    const sendJson = vi.fn();
    useWsStore.setState({ sendJson });
    mockedApiFetch
      .mockResolvedValueOnce(new Response("team-created"))
      .mockResolvedValueOnce(new Response("team-joined"));

    const created = renderHook(() => useCreateTeam(), {
      wrapper: queryClientWrapper(queryClient),
    });
    const joined = renderHook(() => useJoinTeam(), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await expect(created.result.current.mutateAsync("Operations")).resolves.toBe("team-created");
      await expect(joined.result.current.mutateAsync("invite-1")).resolves.toBe("team-joined");
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams", {
      method: "POST",
      body: JSON.stringify({ name: "Operations" }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams/join", {
      method: "POST",
      body: JSON.stringify({ invitation_code: "invite-1" }),
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["teams"] });
    expect(sendJson).toHaveBeenCalledTimes(2);
    expect(sendJson).toHaveBeenCalledWith({ type: "refresh_teams" });
  });

  it("leaves and deletes a team while invalidating the whole affected scope", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    const sendJson = vi.fn();
    useWsStore.setState({ sendJson });
    mockedApiFetch
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

    const leave = renderHook(() => useLeaveTeam("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const remove = renderHook(() => useDeleteTeam("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await leave.result.current.mutateAsync();
      await remove.result.current.mutateAsync();
    });

    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1/leave", { method: "POST" });
    expect(mockedApiFetch).toHaveBeenCalledWith("/api/teams/team-1", { method: "DELETE" });
    for (const queryKey of [
      ["teams"],
      ["team-members", "team-1"],
      ["team-bans", "team-1"],
      ["incidents"],
    ]) {
      expect(invalidate).toHaveBeenCalledWith({ queryKey });
    }
    expect(sendJson).toHaveBeenCalledTimes(2);
  });

  it("transfers management and changes a member role", async () => {
    const queryClient = createTestQueryClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ role: "manager" }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

    const transfer = renderHook(() => useTransferManager("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const setRole = renderHook(() => useSetMemberRole("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await transfer.result.current.mutateAsync("user-2");
      await setRole.result.current.mutateAsync({ userId: "user-3", role: "responder" });
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams/team-1/manager", {
      method: "PUT",
      body: JSON.stringify({ new_manager_id: "user-2" }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams/team-1/members/user-3/role", {
      method: "PUT",
      body: JSON.stringify({ role: "responder" }),
    });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["team-members", "team-1"] });
  });

  it("kicks, bans and unbans members through manager-only endpoints", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(jsonResponse({ kind: "temporary" }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

    const kick = renderHook(() => useKickMember("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const ban = renderHook(() => useBanMember("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const unban = renderHook(() => useUnbanMember("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await act(async () => {
      await kick.result.current.mutateAsync("user-2");
      await ban.result.current.mutateAsync({
        userId: "user-3",
        ban: { kind: "temporary", expires_at: "2026-07-26T12:00:00Z" },
      });
      await unban.result.current.mutateAsync("user-4");
    });

    expect(mockedApiFetch).toHaveBeenNthCalledWith(1, "/api/teams/team-1/members/user-2", {
      method: "DELETE",
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(2, "/api/teams/team-1/bans", {
      method: "POST",
      body: JSON.stringify({
        user_id: "user-3",
        kind: "temporary",
        expires_at: "2026-07-26T12:00:00Z",
      }),
    });
    expect(mockedApiFetch).toHaveBeenNthCalledWith(3, "/api/teams/team-1/bans/user-4", {
      method: "DELETE",
    });
  });

  it("surfaces stable backend codes and mutation fallbacks", async () => {
    const queryClient = createTestQueryClient();
    mockedApiFetch
      .mockResolvedValueOnce(jsonResponse({ code: "manager_cannot_leave" }, 409))
      .mockResolvedValueOnce(new Response("not-json", { status: 500 }));

    const leave = renderHook(() => useLeaveTeam("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });
    const kick = renderHook(() => useKickMember("team-1"), {
      wrapper: queryClientWrapper(queryClient),
    });

    await expect(leave.result.current.mutateAsync()).rejects.toThrow("manager_cannot_leave");
    await expect(kick.result.current.mutateAsync("user-2")).rejects.toThrow("kick_member_failed");
  });
});
