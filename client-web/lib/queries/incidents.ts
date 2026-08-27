import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";
import {
  downloadConversationAttachment,
  type ConversationAttachment,
  type ConversationFeature,
  type PendingConversationAttachment,
} from "../conversations";

export type IncidentStatus = "open" | "acknowledged" | "escalated" | "resolved";
export type IncidentSeverity = "low" | "medium" | "high" | "critical";

export interface Incident {
  id: string;
  team_id: string;
  title: string;
  description: string;
  status: IncidentStatus;
  severity: IncidentSeverity;
  assignee: string | null;
  created_at: string;
  created_by: string | null;
  updated_at: string;
}

export interface IncidentAssignee {
  user_id: string;
  email: string;
}

export interface IncidentListItem extends Omit<Incident, "assignee"> {
  assignee: IncidentAssignee | null;
  unread: boolean;
}

export interface IncidentCounts {
  all: number;
  open: number;
  acknowledged: number;
  escalated: number;
  resolved: number;
}

export interface IncidentListFilters {
  status?: IncidentStatus;
  severity?: IncidentSeverity;
  assignee?: string;
  query?: string;
  sort?: "newest" | "oldest" | "severity";
}

export interface IncidentListResult {
  items: IncidentListItem[];
  counts: IncidentCounts;
}

interface IncidentViewResponse {
  incident_id: string;
  team_id: string;
  title: string;
  description: string;
  status: IncidentStatus;
  severity: IncidentSeverity;
  assignee_id: string | null;
  created_at: string;
  created_by: string | null;
  updated_at: string;
}

interface IncidentListItemResponse extends Omit<IncidentViewResponse, "assignee_id"> {
  assignee: IncidentAssignee | null;
  unread?: boolean;
}

interface IncidentListResponse {
  items: IncidentListItemResponse[];
  counts: IncidentCounts;
}

export interface TimelineReaction {
  emoji: string;
  count: number;
  reacted: boolean;
}

export interface UserSummary {
  user_id: string;
  email: string;
}

export type IncidentActivityItem =
  | {
      type: "system_event";
      id: string;
      kind:
        "created" | "status_changed" | "assigned" | "severity_changed" | "release_step_validated";
      actor: UserSummary | null;
      subject: UserSummary | null;
      data: Record<string, unknown>;
      created_at: string;
    }
  | {
      type: "human_note";
      entry_id: string;
      author: UserSummary | null;
      content: string;
      created_at: string;
      edited_at: string | null;
      attachments?: ConversationAttachment[];
      reactions: TimelineReaction[];
    };

function normalizeIncident(incident: IncidentViewResponse): Incident {
  return {
    id: incident.incident_id,
    team_id: incident.team_id,
    title: incident.title,
    description: incident.description,
    status: incident.status,
    severity: incident.severity,
    assignee: incident.assignee_id,
    created_at: incident.created_at,
    created_by: incident.created_by,
    updated_at: incident.updated_at,
  };
}

function normalizeIncidentListItem(incident: IncidentListItemResponse): IncidentListItem {
  return {
    id: incident.incident_id,
    team_id: incident.team_id,
    title: incident.title,
    description: incident.description,
    status: incident.status,
    severity: incident.severity,
    assignee: incident.assignee,
    created_at: incident.created_at,
    created_by: incident.created_by,
    updated_at: incident.updated_at,
    unread: incident.unread ?? false,
  };
}

function incidentListQuery(teamId: string | undefined, filters: IncidentListFilters) {
  return {
    queryKey: ["incidents", { teamId, ...filters }] as const,
    queryFn: async () => {
      const params = new URLSearchParams();
      if (teamId) params.set("team_id", teamId);
      if (filters.status) params.set("status", filters.status);
      if (filters.severity) params.set("severity", filters.severity);
      if (filters.assignee) params.set("assignee", filters.assignee);
      if (filters.query?.trim()) params.set("q", filters.query.trim());
      if (filters.sort) params.set("sort", filters.sort);

      const res = await apiFetch(`/api/incidents?${params.toString()}`);
      if (!res.ok) throw new Error("incidents_load_failed");
      const response = (await res.json()) as IncidentListResponse;
      return {
        items: response.items.map(normalizeIncidentListItem),
        counts: response.counts,
      } satisfies IncidentListResult;
    },
    enabled: !!teamId,
  };
}

export function useIncidentQueue(teamId: string | undefined, filters: IncidentListFilters) {
  return useQuery(incidentListQuery(teamId, filters));
}

/** Unfiltered incident collection for cross-feature pickers such as Releases. */
export function useIncidents(teamId?: string) {
  return useQuery({
    ...incidentListQuery(teamId, {}),
    select: (result) => result.items,
  });
}

export function useIncident(id: string) {
  return useQuery<Incident>({
    queryKey: ["incident", id],
    queryFn: async () => {
      const res = await apiFetch(`/api/incidents/${id}`);
      if (!res.ok) throw new Error("incident_load_failed");
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
      description,
      severity,
    }: {
      team_id: string;
      title: string;
      description?: string;
      severity: IncidentSeverity;
    }) => {
      const res = await apiFetch("/api/incidents", {
        method: "POST",
        body: JSON.stringify({ team_id, title, description: description ?? "", severity }),
      });
      if (!res.ok) throw new Error("create_incident_failed");
      return res.json(); // usually returns { id: string }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
    },
  });
}

export function useAvailableReactions() {
  return useQuery<string[]>({
    queryKey: ["available-reactions"],
    staleTime: Number.POSITIVE_INFINITY,
    queryFn: async () => {
      const res = await apiFetch("/reactions/available");
      if (!res.ok) throw new Error("reactions_load_failed");
      const body = (await res.json()) as { reactions: string[] };
      return body.reactions;
    },
  });
}

interface IncidentActivityCursor {
  created_at: string;
  id: string;
}

interface IncidentActivityPage {
  items: IncidentActivityItem[];
  next_cursor: IncidentActivityCursor | null;
  features?: ConversationFeature[];
}

/**
 * Incident activity is keyset-paginated: each page carries the cursor of its
 * oldest item, so a long war room can be walked backwards without the page
 * boundary shifting as new notes arrive at the live edge.
 */
export function useIncidentActivity(incidentId: string) {
  const query = useInfiniteQuery<IncidentActivityPage>({
    queryKey: ["activity", incidentId],
    queryFn: async ({ pageParam }) => {
      const cursor = pageParam as IncidentActivityCursor | null;
      const params = new URLSearchParams();
      if (cursor) {
        params.set("before_created_at", cursor.created_at);
        params.set("before_id", cursor.id);
      }
      const query = params.toString();
      const res = await apiFetch(
        `/api/incidents/${incidentId}/activity${query ? `?${query}` : ""}`,
      );
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "activity_load_failed");
      }
      return (await res.json()) as IncidentActivityPage;
    },
    initialPageParam: null,
    getNextPageParam: (page) => page.next_cursor ?? undefined,
  });

  return {
    ...query,
    data: query.data?.pages.flatMap((page) => page.items),
    // Capabilities are a property of the surface, not of a page.
    features: query.data?.pages[0]?.features ?? [],
  };
}

export function useMarkIncidentRead() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      incidentId,
      readThrough,
    }: {
      incidentId: string;
      readThrough: string;
    }) => {
      const response = await apiFetch(`/api/incidents/${incidentId}/read`, {
        method: "PUT",
        body: JSON.stringify({ read_through: readThrough }),
      });
      if (!response.ok) throw new Error("incident_read_failed");
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["incidents"] }),
  });
}

export function useAddTimelineEntry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      incidentId,
      content,
      attachments = [],
    }: {
      incidentId: string;
      content: string;
      attachments?: PendingConversationAttachment[];
    }) => {
      const res = await apiFetch(`/api/incidents/${incidentId}/timeline`, {
        method: "POST",
        body: JSON.stringify({ content, attachments }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "timeline_entry_failed");
      }
      return res.text();
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["activity", variables.incidentId] });
    },
  });
}

export async function downloadTimelineAttachment(attachment: ConversationAttachment) {
  await downloadConversationAttachment(`/api/timeline-attachments/${attachment.id}`, attachment);
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
      queryClient.invalidateQueries({ queryKey: ["activity", variables.incidentId] });
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
      queryClient.invalidateQueries({ queryKey: ["activity", variables.incidentId] });
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
      queryClient.invalidateQueries({ queryKey: ["activity", variables.incidentId] });
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
      queryClient.invalidateQueries({ queryKey: ["activity", variables.incidentId] });
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
