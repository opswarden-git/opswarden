import type { ReleaseListItem, ReleaseState } from "@/lib/queries/releases";

export type ReleaseView = "active" | "blocked" | "completed" | "cancelled" | "all";

export const RELEASE_VIEWS: ReleaseView[] = ["active", "blocked", "completed", "cancelled", "all"];

type HasState = { state: ReleaseState };

/** Returns true for releases that are still ongoing (created, in_progress, or blocked). */
export function isOngoingRelease(release: HasState): boolean {
  return (
    release.state === "created" || release.state === "in_progress" || release.state === "blocked"
  );
}

/** Returns true for releases that can be validated directly (created or in_progress). */
export function isExecutableRelease(release: HasState): boolean {
  return release.state === "created" || release.state === "in_progress";
}

/** Returns true for releases that have reached a terminal state (completed or cancelled). */
export function isTerminalRelease(release: HasState): boolean {
  return release.state === "completed" || release.state === "cancelled";
}

export function normalizeReleaseView(value: string | null): ReleaseView {
  return value && RELEASE_VIEWS.includes(value as ReleaseView) ? (value as ReleaseView) : "active";
}

export function releaseBelongsToView(release: ReleaseListItem, view: ReleaseView) {
  if (view === "all") return true;
  if (view === "active") return isOngoingRelease(release);
  return release.state === view;
}

export function releaseViewCounts(releases: ReleaseListItem[]) {
  return Object.fromEntries(
    RELEASE_VIEWS.map((view) => [
      view,
      releases.filter((release) => releaseBelongsToView(release, view)).length,
    ]),
  ) as Record<ReleaseView, number>;
}
