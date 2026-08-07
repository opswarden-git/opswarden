"use client";

import { Rocket, ShieldAlert } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useLocale, useTranslations } from "next-intl";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { PageTabs } from "@/components/layout/PageTabs";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { Alert } from "@/components/ui/Alert";
import { Link } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useIncidentQueue } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { useAuthStore } from "@/store/auth";
import { TeamHeader } from "./TeamHeader";
import {
  deriveTeamOverview,
  matchesFacet,
  type AttentionFacet,
  type AttentionItem,
} from "./team-overview";

const FACETS: readonly AttentionFacet[] = [
  "all",
  "unacknowledged",
  "assigned",
  "escalated",
  "blocked",
];

/** An unknown `view` falls back to the whole inbox rather than an empty screen. */
function normalizeFacet(value: string | null): AttentionFacet {
  return FACETS.includes(value as AttentionFacet) ? (value as AttentionFacet) : "all";
}

import { formatRelativeAge } from "@/lib/utils";

function AttentionRow({ item, teamId }: { item: AttentionItem; teamId: string }) {
  const t = useTranslations("Teams");
  const locale = useLocale();
  const isIncident = item.resource === "incident";
  const href = isIncident
    ? teamPath(teamId, "incidents", item.id)
    : teamPath(teamId, "releases", item.id);

  return (
    <li>
      <Link
        href={href}
        className="group hover:bg-panel-2 flex min-w-0 gap-3 px-4 py-3 transition-colors sm:px-5"
      >
        <span className="surface-subtle text-muted group-hover:text-gold mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors">
          {isIncident ? (
            <ShieldAlert className="h-4 w-4" aria-hidden="true" />
          ) : (
            <Rocket className="h-4 w-4" aria-hidden="true" />
          )}
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
            <span className="text-text min-w-0 truncate font-medium">{item.title}</span>
            {isIncident ? (
              <>
                <SeverityChip severity={item.severity} />
                <StateChip status={item.status} />
              </>
            ) : (
              <ReleaseStateChip state={item.state} />
            )}
          </span>
          <span className="text-muted mt-1 block text-sm">
            {t(`attentionReasons.${item.reason}`, { related: item.relatedTitle ?? "" })}
          </span>
        </span>
        <time
          dateTime={item.timestamp}
          title={new Date(item.timestamp).toLocaleString(locale)}
          className="text-muted hidden shrink-0 pt-1 text-xs sm:block"
        >
          {formatRelativeAge(item.timestamp, locale)}
        </time>
      </Link>
    </li>
  );
}

export function TeamOverviewPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const searchParams = useSearchParams();
  const userId = useAuthStore((state) => state.user?.id ?? null);
  const { data: teams, isLoading: isLoadingTeams, error: teamsError } = useTeams();
  const {
    data: incidentQueue,
    isLoading: isLoadingIncidents,
    error: incidentsError,
  } = useIncidentQueue(teamId, { sort: "severity" });
  const {
    data: releases,
    isLoading: isLoadingReleases,
    error: releasesError,
  } = useReleases(teamId);
  const team = teams?.find((candidate) => candidate.team_id === teamId);
  const capabilities = deriveCapabilities(team?.role ?? "observer");
  const projection =
    team && incidentQueue && releases
      ? deriveTeamOverview({
          canProgressRelease: capabilities.canProgressRelease,
          incidents: incidentQueue.items,
          releases,
          role: team.role,
          userId,
        })
      : null;
  const state: PageContentState =
    isLoadingTeams || isLoadingIncidents || isLoadingReleases
      ? "loading"
      : teamsError || incidentsError || releasesError || !team || !projection
        ? "error"
        : "ready";

  const base = teamPath(teamId, "overview");
  const activeFacet = normalizeFacet(searchParams.get("view"));
  // Observers hold no assignments, so the facet would always read zero.
  const facets: AttentionFacet[] =
    team?.role === "observer"
      ? ["all", "unacknowledged", "escalated", "blocked"]
      : ["all", "unacknowledged", "assigned", "escalated", "blocked"];
  const items = projection
    ? activeFacet === "all"
      ? projection.attention
      : projection.candidates.filter((item) => matchesFacet(item, activeFacet)).slice(0, 7)
    : [];

  return (
    <PageLayout>
      {team ? <TeamHeader team={team} /> : null}
      <PageContent
        state={state}
        loadingFallback={
          <div className="space-y-6" aria-label={t("loadingOverview")}>
            <div className="surface h-12 animate-pulse rounded-md" />
            <div className="surface h-96 animate-pulse rounded-md" />
          </div>
        }
        errorFallback={<Alert tone="danger">{t("overviewUnavailable")}</Alert>}
      >
        {projection ? (
          <section className="space-y-4" aria-labelledby="attention-title">
            <div>
              <h2 id="attention-title" className="text-text font-semibold">
                {t("needsAttention")}
              </h2>
              <p className="text-muted mt-1 text-sm">{t("needsAttentionDescription")}</p>
            </div>

            {/*
             * Facets, not summary tiles. A count that links somewhere else turns
             * this screen into a table of contents; a count that narrows the
             * queue below keeps it a place where work is picked up. Each one is
             * measured on the same material it filters, so it can never promise
             * more than the click delivers.
             */}
            <PageTabs
              ariaLabel={t("attentionFacetsLabel")}
              tabs={facets.map((facet) => ({
                href: facet === "all" ? base : `${base}?view=${facet}`,
                label: t(`facets.${facet}`),
                count: projection.facetCounts[facet],
                active: facet === activeFacet,
              }))}
            />

            <div className="surface overflow-hidden rounded-md">
              {items.length > 0 ? (
                <ul data-attention-queue="true" className="divide-border divide-y">
                  {items.map((item) => (
                    <AttentionRow key={`${item.resource}-${item.id}`} item={item} teamId={teamId} />
                  ))}
                </ul>
              ) : (
                <div className="px-5 py-10 text-center">
                  <p className="text-text font-medium">{t("nothingNeedsAttention")}</p>
                  <p className="text-muted mt-1 text-sm">{t("nothingNeedsAttentionDescription")}</p>
                </div>
              )}
            </div>
          </section>
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
