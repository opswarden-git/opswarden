import type { TeamRole } from "@/lib/capabilities";
import type { IncidentListItem, IncidentSeverity, IncidentStatus } from "@/lib/queries/incidents";
import type { ReleaseListItem, ReleaseState } from "@/lib/queries/releases";

export type AttentionReason =
  | "criticalUnacknowledged"
  | "assignedUnacknowledged"
  | "unassignedUnacknowledged"
  | "unacknowledged"
  | "assignedEscalation"
  | "activeEscalation"
  | "assignedActive"
  | "releaseBlocked"
  | "releaseReady";

export type AttentionItem =
  | {
      resource: "incident";
      id: string;
      title: string;
      reason: AttentionReason;
      priority: number;
      timestamp: string;
      severity: IncidentSeverity;
      status: IncidentStatus;
      relatedTitle?: undefined;
    }
  | {
      resource: "release";
      id: string;
      title: string;
      reason: "releaseBlocked" | "releaseReady";
      priority: number;
      timestamp: string;
      state: ReleaseState;
      relatedTitle?: string;
      severity?: undefined;
      status?: undefined;
    };

export interface TeamOverviewProjection {
  attention: AttentionItem[];
  assignedIncidents: IncidentListItem[];
  blockedReleases: ReleaseListItem[];
  counts: {
    active: number;
    unacknowledged: number;
    assignedToMe: number;
    escalated: number;
    blockedReleases: number;
  };
}

const severityPriority: Record<IncidentSeverity, number> = {
  critical: 40,
  high: 30,
  medium: 20,
  low: 10,
};

const activeIncident = (incident: IncidentListItem) => incident.status !== "resolved";
const byFreshness = <T extends { updated_at: string }>(left: T, right: T) =>
  new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime();
const byAttentionPriority = (left: AttentionItem, right: AttentionItem) =>
  right.priority - left.priority ||
  new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime();

function selectAttention(candidates: AttentionItem[], limit: number) {
  const ranked = candidates.toSorted(byAttentionPriority);
  const selected = ranked.slice(0, limit);
  const readyRelease = ranked.find((item) => item.reason === "releaseReady");

  // A cross-resource inbox must not silently become an Incident-only queue.
  // Keep the strongest Release action visible while preserving critical work.
  if (
    readyRelease &&
    selected.length === limit &&
    !selected.some((item) => item.id === readyRelease.id && item.resource === "release")
  ) {
    selected[selected.length - 1] = readyRelease;
    selected.sort(byAttentionPriority);
  }
  return selected;
}

function incidentAttention(
  incident: IncidentListItem,
  userId: string | null,
  role: TeamRole,
): AttentionItem | null {
  if (!activeIncident(incident)) return null;
  const assignedToMe = incident.assignee?.user_id === userId;
  const unassigned = incident.assignee === null;
  if (incident.status !== "open" && incident.status !== "escalated" && !assignedToMe) {
    return null;
  }

  let reason: AttentionReason;
  if (assignedToMe && incident.status === "open") reason = "assignedUnacknowledged";
  else if (incident.status === "open" && incident.severity === "critical") {
    reason = "criticalUnacknowledged";
  } else if (incident.status === "open" && unassigned && role === "manager") {
    reason = "unassignedUnacknowledged";
  } else if (incident.status === "open") reason = "unacknowledged";
  else if (assignedToMe && incident.status === "escalated") reason = "assignedEscalation";
  else if (incident.status === "escalated") reason = "activeEscalation";
  else reason = "assignedActive";

  const priority =
    severityPriority[incident.severity] +
    (incident.status === "open" ? 35 : incident.status === "escalated" ? 30 : 5) +
    (assignedToMe ? 20 : 0) +
    (unassigned && role === "manager" ? 10 : 0);

  return {
    resource: "incident",
    id: incident.id,
    title: incident.title,
    reason,
    priority,
    timestamp: incident.updated_at,
    severity: incident.severity,
    status: incident.status,
  };
}

function releaseAttention(
  release: ReleaseListItem,
  canProgressRelease: boolean,
  incidentById: Map<string, IncidentListItem>,
): AttentionItem | null {
  if (release.state === "blocked") {
    const blocker = release.blockers.toSorted(
      (left, right) => severityPriority[right.severity] - severityPriority[left.severity],
    )[0];
    const blockerFreshness = blocker
      ? incidentById.get(blocker.incident_id)?.updated_at
      : undefined;
    const timestamp =
      blockerFreshness && new Date(blockerFreshness) > new Date(release.updated_at)
        ? blockerFreshness
        : release.updated_at;
    return {
      resource: "release",
      id: release.release_id,
      title: release.title,
      reason: "releaseBlocked",
      priority: 90 + (blocker ? severityPriority[blocker.severity] : 0),
      timestamp,
      state: release.state,
      relatedTitle: blocker?.title,
    };
  }

  if (
    canProgressRelease &&
    (release.state === "created" || release.state === "in_progress") &&
    release.next_step
  ) {
    return {
      resource: "release",
      id: release.release_id,
      title: release.title,
      reason: "releaseReady",
      priority: 45,
      timestamp: release.updated_at,
      state: release.state,
      relatedTitle: release.next_step.name,
    };
  }
  return null;
}

export function deriveTeamOverview({
  canProgressRelease,
  incidents,
  releases,
  role,
  userId,
}: {
  canProgressRelease: boolean;
  incidents: IncidentListItem[];
  releases: ReleaseListItem[];
  role: TeamRole;
  userId: string | null;
}): TeamOverviewProjection {
  const active = incidents.filter(activeIncident);
  const assignedIncidents = active
    .filter((incident) => incident.assignee?.user_id === userId)
    .toSorted(byFreshness);
  const blockedReleases = releases
    .filter((release) => release.state === "blocked")
    .toSorted(
      (left, right) => new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime(),
    );
  const incidentById = new Map(incidents.map((incident) => [incident.id, incident]));
  const attentionCandidates = [
    ...active.map((incident) => incidentAttention(incident, userId, role)),
    ...releases.map((release) => releaseAttention(release, canProgressRelease, incidentById)),
  ].filter((item): item is AttentionItem => item !== null);
  const attention = selectAttention(attentionCandidates, 7);

  return {
    attention,
    assignedIncidents,
    blockedReleases,
    counts: {
      active: active.length,
      unacknowledged: active.filter((incident) => incident.status === "open").length,
      assignedToMe: assignedIncidents.length,
      escalated: active.filter((incident) => incident.status === "escalated").length,
      blockedReleases: blockedReleases.length,
    },
  };
}
