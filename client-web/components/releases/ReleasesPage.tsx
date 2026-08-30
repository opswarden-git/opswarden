"use client";

import React from "react";
import { Rocket, Shield } from "lucide-react";
import { useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { CreateReleaseDialog } from "@/components/releases/CreateReleaseDialog";
import { ReleaseTable, ReleaseTableSkeleton } from "@/components/releases/ReleaseTable";
import {
  RELEASE_VIEWS,
  normalizeReleaseView,
  releaseBelongsToView,
  releaseViewCounts,
  type ReleaseView,
} from "@/components/releases/release-views";
import { Alert } from "@/components/ui/Alert";
import { Button, buttonClassNames } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import {
  CollectionSearch,
  MobileCollectionFilters,
  TableFilterControl,
  TableSortControl,
} from "@/components/ui/CollectionControls";
import { Link, usePathname, useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useReleases } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";

export function ReleasesPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Releases");
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const searchParamsString = searchParams.toString();
  const { data: teams, isLoading: isLoadingTeams, error: teamsError } = useTeams();
  const { data: releases, isLoading, error } = useReleases(teamId);
  const view = normalizeReleaseView(searchParams.get("view"));
  const sort = searchParams.get("sort") === "oldest" ? "oldest" : "newest";
  const urlQuery = searchParams.get("q") ?? "";
  const activeTeam = teams?.find((team) => team.team_id === teamId);
  const role = activeTeam?.role ?? "observer";
  const capabilities = deriveCapabilities(role);
  const hasNoTeams = teams?.length === 0;
  const counts = releaseViewCounts(releases ?? []);
  const visibleReleases = (releases ?? [])
    .filter((release) => releaseBelongsToView(release, view))
    .filter((release) => {
      const query = urlQuery.trim().toLocaleLowerCase();
      return (
        !query ||
        release.title.toLocaleLowerCase().includes(query) ||
        release.release_id.toLocaleLowerCase().includes(query)
      );
    })
    .toSorted((left, right) => {
      const delta = new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
      return sort === "newest" ? delta : -delta;
    });
  const hasReleases = (releases?.length ?? 0) > 0;

  const paramsWith = (changes: Record<string, string | undefined>) => {
    const params = new URLSearchParams(searchParams.toString());
    for (const [name, value] of Object.entries(changes)) {
      if (value) params.set(name, value);
      else params.delete(name);
    }
    const suffix = params.toString();
    return suffix ? `${pathname}?${suffix}` : pathname;
  };

  const releaseHref = (releaseId: string) => {
    const detailPath = teamPath(teamId, "releases", releaseId);
    return view === "active" ? detailPath : `${detailPath}?view=${view}`;
  };

  const setParam = (name: string, value?: string) => router.push(paramsWith({ [name]: value }));
  const commitSearch = React.useCallback(
    (value: string) => {
      const params = new URLSearchParams(searchParamsString);
      const normalized = value.trim();
      if (normalized) params.set("q", normalized);
      else params.delete("q");
      const suffix = params.toString();
      router.replace(suffix ? `${pathname}?${suffix}` : pathname);
    },
    [pathname, router, searchParamsString],
  );
  const viewLabel = (value: ReleaseView) => t(`view${value[0].toUpperCase()}${value.slice(1)}`);
  const activeFilterCount = (view === "all" ? 0 : 1) + (sort === "newest" ? 0 : 1);
  const clearFilters = () =>
    router.push(paramsWith({ view: "all", sort: undefined, q: undefined }));
  const headers = {
    colStatus: (
      <TableFilterControl
        label={t("colStatus")}
        value={view}
        activeLabel={viewLabel(view)}
        onChange={(value) => setParam("view", value)}
        options={RELEASE_VIEWS.map((value) => ({
          value,
          label: `${viewLabel(value)} (${counts[value]})`,
        }))}
      />
    ),
    colAge: (
      <TableSortControl
        label={t("colAge")}
        direction={sort === "newest" ? "ascending" : "descending"}
        onToggle={() => setParam("sort", sort === "newest" ? "oldest" : undefined)}
      />
    ),
  };

  const contentState: PageContentState =
    isLoadingTeams || isLoading
      ? "loading"
      : teamsError || error
        ? "error"
        : hasNoTeams || visibleReleases.length === 0
          ? "empty"
          : "ready";

  return (
    <PageLayout>
      <PageHeader
        actions={
          !hasNoTeams ? (
            <>
              <CollectionSearch
                key={urlQuery}
                initialValue={urlQuery}
                label={t("searchLabel")}
                placeholder={t("searchPlaceholder")}
                onCommit={commitSearch}
              />
              <MobileCollectionFilters
                activeCount={activeFilterCount}
                label={t("filtersLabel")}
                title={t("filtersLabel")}
                clearLabel={t("clearFilters")}
                closeLabel={t("close")}
                doneLabel={t("done")}
                onClear={clearFilters}
              >
                <label className="space-y-2">
                  <span className="text-muted block text-sm uppercase">{t("colStatus")}</span>
                  <select
                    value={view}
                    onChange={(event) => setParam("view", event.target.value)}
                    className="ow-input h-10 w-full rounded-md px-3 text-sm"
                  >
                    {RELEASE_VIEWS.map((value) => (
                      <option key={value} value={value}>
                        {viewLabel(value)} ({counts[value]})
                      </option>
                    ))}
                  </select>
                </label>
                <label className="space-y-2">
                  <span className="text-muted block text-sm uppercase">{t("sortLabel")}</span>
                  <select
                    value={sort}
                    onChange={(event) => setParam("sort", event.target.value)}
                    className="ow-input h-10 w-full rounded-md px-3 text-sm"
                  >
                    <option value="newest">{t("sortNewest")}</option>
                    <option value="oldest">{t("sortOldest")}</option>
                  </select>
                </label>
              </MobileCollectionFilters>
              {capabilities.canCreateRelease ? <CreateReleaseDialog teamId={teamId} /> : null}
            </>
          ) : null
        }
      />

      <PageContent
        state={contentState}
        loadingFallback={<ReleaseTableSkeleton />}
        errorFallback={<Alert tone="danger">{t("failedToLoad")}</Alert>}
        emptyFallback={
          hasNoTeams ? (
            <EmptyState
              icon={<Shield className="h-6 w-6" />}
              title={t("noTeamsYet")}
              description={t("noTeamsDesc")}
              action={
                <Link href="/teams" className={buttonClassNames({ variant: "primary", size: "lg" })}>
                  {t("goToTeams")}
                </Link>
              }
            />
          ) : (
            <EmptyState
              icon={<Rocket className="h-6 w-6" />}
              title={hasReleases ? t("noMatchingReleases") : t("noReleasesYet")}
              description={hasReleases ? t("noMatchingReleasesDesc") : t("noReleasesDesc")}
              action={
                hasReleases ? (
                  <Button onClick={clearFilters}>{t("clearFilters")}</Button>
                ) : null
              }
            />
          )
        }
      >
        <ReleaseTable
          releases={visibleReleases}
          hrefFor={releaseHref}
          headers={headers}
          ageSortDirection={sort === "newest" ? "ascending" : "descending"}
        />
      </PageContent>
    </PageLayout>
  );
}
