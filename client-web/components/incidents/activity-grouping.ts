import type { IncidentActivityItem } from "@/lib/queries/incidents";

/**
 * How long a run of notes from one author stays open.
 *
 * Mattermost collapses consecutive posts within five minutes
 * (`POST_COLLAPSE_TIMEOUT`). Two is a deliberate departure: during an incident
 * messages arrive in bursts, and a five-minute window would swallow distinct
 * turns of a conversation into a single block.
 */
export const GROUPING_WINDOW_MS = 2 * 60 * 1000;

/**
 * Does this item continue the block started by the one before it?
 *
 * Consecutive notes from the same author collapse into one block — a single
 * avatar, a single name, a single timestamp. That is what makes a transcript
 * read as a conversation rather than a log.
 */
export function groupsWithPrevious(
  current: IncidentActivityItem,
  previous: IncidentActivityItem | undefined,
): boolean {
  // Only notes group. A system event between two notes therefore breaks the run
  // on its own, which is the whole point: a status change, an assignment or an
  // escalation is exactly what someone re-reading an incident is looking for,
  // and it must never be absorbed into a series of notes.
  if (!previous || current.type !== "human_note" || previous.type !== "human_note") return false;

  // A null author is a deleted account, not an identity. Two of them cannot be
  // shown to be the same person, so they never merge.
  const author = current.author?.user_id;
  if (!author || author !== previous.author?.user_id) return false;

  const elapsed = new Date(current.created_at).getTime() - new Date(previous.created_at).getTime();
  return Number.isFinite(elapsed) && elapsed >= 0 && elapsed <= GROUPING_WINDOW_MS;
}

/**
 * Resolve, for each item, whether the block continues above and below it. The
 * pair drives the visual seam: only the first item of a run carries the header,
 * only the last one closes the block.
 */
export function resolveGrouping(items: IncidentActivityItem[]) {
  const continuesAbove = items.map((item, index) => groupsWithPrevious(item, items[index - 1]));
  return items.map((_, index) => ({
    continuesAbove: continuesAbove[index],
    continuesBelow: continuesAbove[index + 1] ?? false,
  }));
}
