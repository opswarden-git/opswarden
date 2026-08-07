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

/**
 * Views onto the one inbox, not separate lists. A facet narrows what is already
 * ranked; it never opens a second queue, because the cross-resource inbox is
 * what says this product is about Incidents *and* Releases.
 *
 * They overlap on purpose: an unacknowledged Incident assigned to you answers
 * both `unacknowledged` and `assigned`.
 */
export type AttentionFacet = "all" | "unacknowledged" | "assigned" | "escalated" | "blocked";

export function matchesFacet(item: AttentionItem, facet: AttentionFacet): boolean {
  switch (facet) {
    case "all":
      return true;
    case "unacknowledged":
      return item.resource === "incident" && item.status === "open";
    case "assigned":
      return item.reason.startsWith("assigned");
    case "escalated":
      return item.resource === "incident" && item.status === "escalated";
    case "blocked":
      return item.reason === "releaseBlocked";
  }
}

export interface TeamOverviewProjection {
  /** The `all` facet: ranked, capped, with the Release guard applied. */
  attention: AttentionItem[];
  /** Ranked and uncapped, so a facet narrows the same material. */
  candidates: AttentionItem[];
  /**
   * Counted over `candidates`, never over the raw lists. A facet that announced
   * more than clicking it shows would be a silent lie.
   */
  facetCounts: Record<AttentionFacet, number>;
}

const severityPriority: Record<IncidentSeverity, number> = {
  critical: 40,
  high: 30,
  medium: 20,
  low: 10,
};

const activeIncident = (incident: IncidentListItem) => incident.status !== "resolved";
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
  const incidentById = new Map(incidents.map((incident) => [incident.id, incident]));
  const candidates = [
    ...active.map((incident) => incidentAttention(incident, userId, role)),
    ...releases.map((release) => releaseAttention(release, canProgressRelease, incidentById)),
  ]
    .filter((item): item is AttentionItem => item !== null)
    .toSorted(byAttentionPriority);

  return {
    attention: selectAttention(candidates, 7),
    candidates,
    facetCounts: {
      all: candidates.length,
      unacknowledged: candidates.filter((item) => matchesFacet(item, "unacknowledged")).length,
      assigned: candidates.filter((item) => matchesFacet(item, "assigned")).length,
      escalated: candidates.filter((item) => matchesFacet(item, "escalated")).length,
      blocked: candidates.filter((item) => matchesFacet(item, "blocked")).length,
    },
  };
}
