import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";
import { useWsStore } from "../ws";
import type { TeamRole } from "../capabilities";

/** Tell the server to re-resolve this connection's team scope after the current
 *  user's membership changed (create/join/leave/delete), so team presence and
 *  in-band authz update without waiting for a reconnect. Dropped if the socket
 *  is closed — a reconnect re-resolves teams anyway. */
function notifyTeamMembershipChanged() {
  useWsStore.getState().sendJson({ type: "refresh_teams" });
}

export interface Team {
  team_id: string;
  name: string;
  role: TeamRole;
  created_at: string;
  member_count: number;
  active_incident_count: number;
  active_release_count: number;
  blocked_release_count: number;
}

export interface TeamMember {
  user_id: string;
  email: string;
  role: TeamRole;
  joined_at: string;
}

export interface TeamBan {
  user: { user_id: string; email: string };
  kind: "temporary" | "permanent";
  expires_at: string | null;
  reason: string | null;
  moderator: { user_id: string; email: string } | null;
  created_at: string;
  active: boolean;
}

export function useTeams() {
  return useQuery<Team[]>({
    queryKey: ["teams"],
    queryFn: async () => {
      const res = await apiFetch("/api/teams");
      if (!res.ok) throw new Error("Failed to fetch teams");
      return res.json();
    },
  });
}

export function useTeamMembers(teamId: string | undefined) {
  return useQuery<TeamMember[]>({
    queryKey: ["team-members", teamId],
    queryFn: async () => {
      if (!teamId) return [];
      const res = await apiFetch(`/api/teams/${teamId}/members`);
      if (!res.ok) throw new Error("Failed to fetch team members");
      return res.json();
    },
    enabled: !!teamId,
  });
}

export function useInvitationCode(teamId: string | undefined, enabled: boolean) {
  return useQuery<{ invitation_code: string }>({
    queryKey: ["team-invitation", teamId],
    queryFn: async () => {
      const res = await apiFetch(`/api/teams/${teamId}/invitation`);
      if (!res.ok) throw new Error("invitation_code_failed");
      return res.json();
    },
    enabled: !!teamId && enabled,
  });
}

export function useTeamBans(teamId: string | undefined, enabled: boolean) {
  return useQuery<TeamBan[]>({
    queryKey: ["team-bans", teamId],
    queryFn: async () => {
      const res = await apiFetch(`/api/teams/${teamId}/bans`);
      if (!res.ok) throw new Error("list_bans_failed");
      return res.json();
    },
    enabled: !!teamId && enabled,
  });
}

export function useCreateTeam() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (name: string) => {
      const res = await apiFetch("/api/teams", {
        method: "POST",
        body: JSON.stringify({ name }),
      });
      if (!res.ok) throw new Error("Failed to create team");
      return res.text(); // returns ID or empty
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["teams"] });
      notifyTeamMembershipChanged();
    },
  });
}

export function useJoinTeam() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (invitation_code: string) => {
      const res = await apiFetch("/api/teams/join", {
        method: "POST",
        body: JSON.stringify({ invitation_code }),
      });
      if (!res.ok) throw new Error("Failed to join team. Check your code.");
      return res.text();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["teams"] });
      notifyTeamMembershipChanged();
    },
  });
}

/**
 * Team membership/ownership mutations. On error the thrown message is the
 * backend's stable error `code` (e.g. `manager_cannot_leave`, `not_manager`) so
 * the UI maps it through `errors.<code>` i18n. Each invalidates the team list,
 * the affected roster, and incident views the membership change touches.
 */
function invalidateTeamScope(queryClient: ReturnType<typeof useQueryClient>, teamId: string) {
  queryClient.invalidateQueries({ queryKey: ["teams"] });
  queryClient.invalidateQueries({ queryKey: ["team-members", teamId] });
  queryClient.invalidateQueries({ queryKey: ["team-bans", teamId] });
  queryClient.invalidateQueries({ queryKey: ["incidents"] });
}

export function useLeaveTeam(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const res = await apiFetch(`/api/teams/${teamId}/leave`, { method: "POST" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "leave_team_failed");
      }
    },
    onSuccess: () => {
      invalidateTeamScope(queryClient, teamId);
      notifyTeamMembershipChanged();
    },
  });
}

export function useDeleteTeam(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const res = await apiFetch(`/api/teams/${teamId}`, { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "delete_team_failed");
      }
    },
    onSuccess: () => {
      invalidateTeamScope(queryClient, teamId);
      notifyTeamMembershipChanged();
    },
  });
}

export function useTransferManager(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (newManagerId: string) => {
      const res = await apiFetch(`/api/teams/${teamId}/manager`, {
        method: "PUT",
        body: JSON.stringify({ new_manager_id: newManagerId }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "transfer_manager_failed");
      }
      return res.json();
    },
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
  });
}

/** Promote/demote a member between Observer and Responder (Manager-only server-side). */
export function useSetMemberRole(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ userId, role }: { userId: string; role: "observer" | "responder" }) => {
      const res = await apiFetch(`/api/teams/${teamId}/members/${userId}/role`, {
        method: "PUT",
        body: JSON.stringify({ role }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "set_member_role_failed");
      }
    },
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
  });
}

/** Kick a member out of the team (Manager-only server-side). */
export function useKickMember(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (userId: string) => {
      const res = await apiFetch(`/api/teams/${teamId}/members/${userId}`, { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "kick_member_failed");
      }
    },
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
  });
}

/** A ban request: permanent, or temporary with an ISO `expires_at`. */
export type BanKindInput = { kind: "permanent" } | { kind: "temporary"; expires_at: string };

/** Ban a member from the team (Manager-only). Removes their membership server-side. */
export function useBanMember(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ userId, ban }: { userId: string; ban: BanKindInput }) => {
      const res = await apiFetch(`/api/teams/${teamId}/bans`, {
        method: "POST",
        body: JSON.stringify({ user_id: userId, ...ban }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "ban_member_failed");
      }
      return res.json();
    },
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
  });
}

export function useUnbanMember(teamId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (userId: string) => {
      const res = await apiFetch(`/api/teams/${teamId}/bans/${userId}`, { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "unban_member_failed");
      }
    },
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
  });
}
