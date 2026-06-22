import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

export type IncidentStatus = "open" | "acknowledged" | "escalated" | "resolved";
export type IncidentSeverity = "low" | "medium" | "high" | "critical";

export interface Incident {
  id: string;
  team_id: string;
  title: string;
  status: IncidentStatus;
  severity: IncidentSeverity;
  assignee: string | null;
  created_at: string;
}

interface IncidentViewResponse {
  incident_id: string;
  team_id: string;
  title: string;
  status: IncidentStatus;
  severity: IncidentSeverity;
  assignee_id: string | null;
  created_at: string;
}

export interface TimelineReaction {
  emoji: string;
  count: number;
  reacted: boolean;
}

export interface TimelineEntry {
  id: string;
  incident_id: string;
  author_id: string;
  content: string;
  created_at: string;
  edited_at: string | null;
  reactions: TimelineReaction[];
}

interface TimelineEntryResponse {
  entry_id: string;
  incident_id: string;
  author_id: string;
  content: string;
  created_at: string;
  edited_at: string | null;
  reactions: TimelineReaction[];
}

interface TimelineResponse {
  entries: TimelineEntryResponse[];
}

function normalizeIncident(incident: IncidentViewResponse): Incident {
  return {
    id: incident.incident_id,
    team_id: incident.team_id,
    title: incident.title,
    status: incident.status,
    severity: incident.severity,
    assignee: incident.assignee_id,
    created_at: incident.created_at,
  };
}

function normalizeTimelineEntry(entry: TimelineEntryResponse): TimelineEntry {
  return {
    id: entry.entry_id,
    incident_id: entry.incident_id,
    author_id: entry.author_id,
    content: entry.content,
    created_at: entry.created_at,
    edited_at: entry.edited_at,
    reactions: entry.reactions ?? [],
  };
}

export function useIncidents(teamId?: string) {
  return useQuery<Incident[]>({
    queryKey: ["incidents", { teamId }],
    queryFn: async () => {
      const url = teamId ? `/api/incidents?team_id=${teamId}` : `/api/incidents`;
      const res = await apiFetch(url);
      if (!res.ok) throw new Error("Failed to fetch incidents");
      const incidents = (await res.json()) as IncidentViewResponse[];
      return incidents.map(normalizeIncident);
    },
    enabled: !!teamId, // typically we only load when we select a team
  });
}

export function useIncident(id: string) {
  return useQuery<Incident>({
    queryKey: ["incident", id],
    queryFn: async () => {
      const res = await apiFetch(`/api/incidents/${id}`);
      if (!res.ok) throw new Error("Failed to fetch incident");
      const incident = (await res.json()) as IncidentViewResponse;
      return normalizeIncident(incident);
    },
  });
}

export function useCreateIncident() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      team_id,
      title,
      severity,
    }: {
      team_id: string;
      title: string;
      severity: IncidentSeverity;
    }) => {
      const res = await apiFetch("/api/incidents", {
        method: "POST",
        body: JSON.stringify({ team_id, title, severity }),
      });
      if (!res.ok) throw new Error("Failed to create incident");
      return res.json(); // usually returns { id: string }
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["incidents", { teamId: variables.team_id }] });
    },
  });
}

export function useTimeline(incidentId: string) {
  return useQuery<{ entries: TimelineEntry[] }>({
    queryKey: ["timeline", incidentId],
    queryFn: async () => {
      const res = await apiFetch(`/api/incidents/${incidentId}/timeline`);
      if (!res.ok) throw new Error("Failed to fetch timeline");
      const timeline = (await res.json()) as TimelineResponse;
      return { entries: timeline.entries.map(normalizeTimelineEntry) };
    },
  });
}

export function useAddTimelineEntry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ incidentId, content }: { incidentId: string; content: string }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/timeline`, {
        method: "POST",
        body: JSON.stringify({ content }),
      });
      if (!res.ok) throw new Error("Failed to post timeline entry");
      return res.text();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["timeline", variables.incidentId] });
    },
  });
}

/** Edit a timeline entry's content (author-only, enforced server-side). */
export function useEditTimelineEntry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      incidentId,
      entryId,
      content,
    }: {
      incidentId: string;
      entryId: string;
      content: string;
    }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/timeline/${entryId}`, {
        method: "PUT",
        body: JSON.stringify({ content }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "edit_timeline_entry_failed");
      }
      return res.json();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["timeline", variables.incidentId] });
    },
  });
}

/** Toggle the current user's emoji reaction on a timeline entry (any member). */
export function useToggleTimelineReaction() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      incidentId,
      entryId,
      emoji,
    }: {
      incidentId: string;
      entryId: string;
      emoji: string;
    }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/timeline/${entryId}/reactions`, {
        method: "POST",
        body: JSON.stringify({ emoji }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "toggle_reaction_failed");
      }
      return res.json();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["timeline", variables.incidentId] });
    },
  });
}

export function useUpdateIncidentStatus() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      incidentId,
      status,
    }: {
      incidentId: string;
      status: Exclude<IncidentStatus, "open">;
    }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/status`, {
        method: "PUT",
        body: JSON.stringify({ status }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "status_update_failed");
      }
      return res.json();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["incident", variables.incidentId] });
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
    },
  });
}

export function useAssignIncident() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ incidentId, assigneeId }: { incidentId: string; assigneeId: string }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/assign`, {
        method: "PUT",
        body: JSON.stringify({ assignee_id: assigneeId }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "assign_failed");
      }
      return res.json();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["incident", variables.incidentId] });
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
    },
  });
}

export function useDeleteIncident() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (incidentId: string) => {
      const res = await apiFetch(`/api/incidents/${incidentId}`, { method: "DELETE" });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "delete_incident_failed");
      }
    },
    onSuccess: (_data, incidentId) => {
      queryClient.removeQueries({ queryKey: ["incident", incidentId] });
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
    },
  });
}
