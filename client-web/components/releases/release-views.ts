import type { ReleaseListItem } from "@/lib/queries/releases";

export type ReleaseView = "active" | "blocked" | "completed" | "cancelled" | "all";

export const RELEASE_VIEWS: ReleaseView[] = ["active", "blocked", "completed", "cancelled", "all"];

export function normalizeReleaseView(value: string | null): ReleaseView {
  return value && RELEASE_VIEWS.includes(value as ReleaseView) ? (value as ReleaseView) : "active";
}

export function releaseBelongsToView(release: ReleaseListItem, view: ReleaseView) {
  if (view === "all") return true;
  if (view === "active") return release.state === "created" || release.state === "in_progress";
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
