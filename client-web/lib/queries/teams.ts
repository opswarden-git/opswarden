import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

export interface Team {
  team_id: string;
  name: string;
  invitation_code?: string;
  role: "manager" | "responder" | "observer";
}

export interface TeamMember {
  user_id: string;
  email: string;
  role: "manager" | "responder" | "observer";
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
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
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
    onSuccess: () => invalidateTeamScope(queryClient, teamId),
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
