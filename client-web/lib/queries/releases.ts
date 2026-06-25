import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

export type ReleaseState = "created" | "in_progress" | "blocked" | "completed" | "cancelled";

export interface ReleaseStep {
  name: string;
  validated: boolean;
  validated_by: string | null;
  validated_at: string | null;
}

export interface Release {
  release_id: string;
  team_id: string;
  title: string;
  /** Effective state (with `blocked` already resolved from linked incidents). */
  state: ReleaseState;
  steps: ReleaseStep[];
  linked_incident_ids: string[];
  created_at: string;
}

export function useReleases(teamId: string) {
  return useQuery<Release[]>({
    queryKey: ["releases", { teamId }],
    queryFn: async () => {
      const res = await apiFetch(`/api/releases?team_id=${teamId}`);
      if (!res.ok) throw new Error("Failed to fetch releases");
      return res.json();
    },
    enabled: !!teamId,
  });
}

export function useRelease(releaseId: string | undefined) {
  return useQuery<Release>({
    queryKey: ["release", releaseId],
    queryFn: async () => {
      const res = await apiFetch(`/api/releases/${releaseId}`);
      if (!res.ok) throw new Error("Failed to fetch release");
      return res.json();
    },
    enabled: !!releaseId,
  });
}

const releasesKey = (teamId: string) => ["releases", { teamId }] as const;

/** Invalidate the team's release list plus a specific release's detail. */
function refreshRelease(
  queryClient: ReturnType<typeof useQueryClient>,
  teamId: string,
  releaseId?: string,
) {
  queryClient.invalidateQueries({ queryKey: releasesKey(teamId) });
  if (releaseId) queryClient.invalidateQueries({ queryKey: ["release", releaseId] });
}

async function failWithCode(res: Response, fallback: string): Promise<never> {
  const body = await res.json().catch(() => null);
  throw new Error(body?.code ?? fallback);
}

export function useCreateRelease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      team_id,
      title,
      steps,
    }: {
      team_id: string;
      title: string;
      steps: string[];
    }) => {
      const res = await apiFetch("/api/releases", {
        method: "POST",
        body: JSON.stringify({ team_id, title, steps }),
      });
      if (!res.ok) return failWithCode(res, "create_release_failed");
      return res.json() as Promise<Release>;
    },
    onSuccess: (created, variables) => {
      queryClient.setQueryData<Release[]>(releasesKey(variables.team_id), (current) => {
        if (!current) return [created];
        if (current.some((release) => release.release_id === created.release_id)) return current;
        return [created, ...current];
      });
      queryClient.setQueryData(["release", created.release_id], created);
      queryClient.invalidateQueries({ queryKey: releasesKey(variables.team_id) });
    },
  });
}

export function useValidateStep() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      releaseId,
      step,
    }: {
      releaseId: string;
      step: string;
      teamId: string;
    }) => {
      const res = await apiFetch(
        `/api/releases/${releaseId}/steps/${encodeURIComponent(step)}/validate`,
        { method: "POST" },
      );
      if (!res.ok) return failWithCode(res, "validate_step_failed");
      return res.json() as Promise<Release>;
    },
    onSuccess: (_data, variables) =>
      refreshRelease(queryClient, variables.teamId, variables.releaseId),
  });
}

export function useLinkIncident() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      releaseId,
      incidentId,
    }: {
      releaseId: string;
      incidentId: string;
      teamId: string;
    }) => {
      const res = await apiFetch(`/api/releases/${releaseId}/incidents/${incidentId}/link`, {
        method: "POST",
      });
      if (!res.ok) return failWithCode(res, "link_incident_failed");
      return res.json() as Promise<Release>;
    },
    onSuccess: (_data, variables) =>
      refreshRelease(queryClient, variables.teamId, variables.releaseId),
  });
}

export function useUnlinkIncident() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      releaseId,
      incidentId,
    }: {
      releaseId: string;
      incidentId: string;
      teamId: string;
    }) => {
      const res = await apiFetch(`/api/releases/${releaseId}/incidents/${incidentId}/link`, {
        method: "DELETE",
      });
      if (!res.ok) return failWithCode(res, "unlink_incident_failed");
      return res.json() as Promise<Release>;
    },
    onSuccess: (_data, variables) =>
      refreshRelease(queryClient, variables.teamId, variables.releaseId),
  });
}

export function useCancelRelease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ releaseId }: { releaseId: string; teamId: string }) => {
      const res = await apiFetch(`/api/releases/${releaseId}/cancel`, { method: "POST" });
      if (!res.ok) return failWithCode(res, "cancel_release_failed");
      return res.json() as Promise<Release>;
    },
    onSuccess: (_data, variables) =>
      refreshRelease(queryClient, variables.teamId, variables.releaseId),
  });
}
