"use client";

import React from "react";
import { Rocket, Shield } from "lucide-react";
import { useTranslations } from "next-intl";
import { useSearchParams } from "next/navigation";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { PageTabs } from "@/components/layout/PageTabs";
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
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
  const { data: releases, isLoading, error } = useReleases(teamId);
  const selectedReleaseId = searchParams.get("release") ?? "";
  const view = normalizeReleaseView(searchParams.get("view"));

  const activeTeam = teams?.find((team) => team.team_id === teamId);
  const role = activeTeam?.role ?? "observer";
  const capabilities = deriveCapabilities(role);
  const hasNoTeams = teams?.length === 0;
  const counts = releaseViewCounts(releases ?? []);
  const visibleReleases = (releases ?? []).filter((release) => releaseBelongsToView(release, view));
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

  const tabHref = (nextView: ReleaseView) =>
    paramsWith({ view: nextView === "active" ? undefined : nextView, release: undefined });
  const releaseHref = (releaseId: string) => {
    const detailPath = teamPath(teamId, "releases", releaseId);
    return view === "active" ? detailPath : `${detailPath}?view=${view}`;
  };
  const legacyDetailHref = selectedReleaseId ? releaseHref(selectedReleaseId) : null;

  React.useEffect(() => {
    if (!legacyDetailHref) return;
    router.replace(legacyDetailHref);
  }, [legacyDetailHref, router]);

  const contentState: PageContentState =
    isLoadingTeams || isLoading
      ? "loading"
      : error
        ? "error"
        : hasNoTeams || visibleReleases.length === 0
          ? "empty"
          : "ready";

  return (
    <PageLayout>
      <PageHeader
        context={
          isLoadingTeams ? (
            <span className="bg-muted/20 inline-block h-4 w-24 animate-pulse rounded" />
          ) : activeTeam ? (
            <Link
              href={teamPath(teamId, "overview")}
              className="hover:text-text hover:underline transition-colors"
            >
              {activeTeam.name}
            </Link>
          ) : null
        }
        title={t("title")}
        description={t("queueDescription")}
        actions={capabilities.canCreateRelease ? <CreateReleaseDialog teamId={teamId} /> : null}
      />

      {!hasNoTeams ? (
        <PageTabs
          ariaLabel={t("viewsLabel")}
          tabs={RELEASE_VIEWS.map((tab) => ({
            href: tabHref(tab),
            label: t(`view${tab[0].toUpperCase()}${tab.slice(1)}`),
            count: counts[tab],
            active: view === tab,
          }))}
        />
      ) : null}

      <PageContent
        state={contentState}
        loadingFallback={<ReleaseTableSkeleton />}
        errorFallback={<Alert tone="danger">{t("failedToLoad")}</Alert>}
        emptyFallback={
          hasNoTeams ? (
            <div className="surface rounded-md p-12 text-center">
              <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
              <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
              <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
              <Link href="/teams" className={buttonClassNames({ variant: "primary", size: "lg" })}>
                {t("goToTeams")}
              </Link>
            </div>
          ) : (
            <div className="surface rounded-md p-12 text-center">
              <Rocket className="text-muted/50 mx-auto mb-4 h-12 w-12" />
              <h3 className="text-text text-lg font-medium">
                {hasReleases ? t("noMatchingReleases") : t("noReleasesYet")}
              </h3>
              <p className="text-muted mt-2 text-sm">
                {hasReleases ? t("noMatchingReleasesDesc") : t("noReleasesDesc")}
              </p>
              {hasReleases ? (
                <Button className="mt-6" onClick={() => router.push(pathname)}>
                  {t("clearFilters")}
                </Button>
              ) : null}
            </div>
          )
        }
      >
        <ReleaseTable releases={visibleReleases} hrefFor={releaseHref} />
      </PageContent>
    </PageLayout>
  );
}
