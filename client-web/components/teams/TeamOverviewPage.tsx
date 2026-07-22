"use client";

import { Activity, CircleAlert, Rocket, ShieldAlert, UserRoundCheck } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
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
import { deriveTeamOverview, type AttentionItem } from "./team-overview";

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
        className="group hover:bg-panel-2 flex min-w-0 gap-3 px-4 py-3.5 transition-colors sm:px-5"
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

  const incidentBase = teamPath(teamId, "incidents");
  const summary = projection
    ? [
        {
          label: t("unacknowledged"),
          description: t("unacknowledgedDescription"),
          value: projection.counts.unacknowledged,
          icon: CircleAlert,
          href: incidentBase,
        },
        team?.role === "observer"
          ? {
              label: t("activeIncidents"),
              description: t("activeIncidentsDescription"),
              value: projection.counts.active,
              icon: ShieldAlert,
              href: `${incidentBase}?view=all`,
            }
          : {
              label: t("assignedToMe"),
              description: t("assignedToMeDescription"),
              value: projection.counts.assignedToMe,
              icon: UserRoundCheck,
              href: `${incidentBase}?view=all&assignee=${userId ?? ""}`,
            },
        {
          label: t("activeEscalations"),
          description: t("activeEscalationsDescription"),
          value: projection.counts.escalated,
          icon: Activity,
          href: `${incidentBase}?view=escalated`,
        },
        {
          label: t("blockedReleases"),
          description: t("blockedReleasesDescription"),
          value: projection.counts.blockedReleases,
          icon: Rocket,
          href: `${teamPath(teamId, "releases")}?view=blocked`,
        },
      ]
    : [];

  return (
    <PageLayout>
      {team ? <TeamHeader team={team} /> : null}
      <PageContent
        state={state}
        loadingFallback={
          <div className="space-y-6" aria-label={t("loadingOverview")}>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              {Array.from({ length: 4 }, (_, index) => (
                <div key={index} className="surface h-28 animate-pulse rounded-md" />
              ))}
            </div>
            <div className="surface h-96 animate-pulse rounded-md" />
          </div>
        }
        errorFallback={<Alert tone="danger">{t("overviewUnavailable")}</Alert>}
      >
        {projection ? (
          <div className="space-y-6">
            <section aria-labelledby="overview-summary-title">
              <div className="mb-3">
                <h2 id="overview-summary-title" className="text-text font-semibold">
                  {t("operationalSummary")}
                </h2>
                <p className="text-muted mt-1 text-sm">{t("operationalSummaryDescription")}</p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                {summary.map((item) => (
                  <Link
                    key={item.label}
                    href={item.href}
                    className="surface group rounded-md p-4 transition-colors hover:bg-white/[0.04]"
                  >
                    <span className="flex items-start justify-between gap-3">
                      <span className="min-w-0">
                        <span className="text-text block text-sm font-medium">{item.label}</span>
                        <span className="text-muted mt-1 block text-xs">{item.description}</span>
                      </span>
                      <item.icon
                        className="text-muted group-hover:text-gold h-4 w-4 shrink-0 transition-colors"
                        aria-hidden="true"
                      />
                    </span>
                    <span className="text-text mt-4 block text-2xl font-semibold tabular-nums">
                      {item.value}
                    </span>
                  </Link>
                ))}
              </div>
            </section>

            <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_20rem]">
              <section
                className="surface overflow-hidden rounded-md"
                aria-labelledby="attention-title"
              >
                <div className="border-border border-b px-4 py-4 sm:px-5">
                  <h2 id="attention-title" className="text-text font-semibold">
                    {t("needsAttention")}
                  </h2>
                  <p className="text-muted mt-1 text-sm">{t("needsAttentionDescription")}</p>
                </div>
                {projection.attention.length > 0 ? (
                  <ul className="divide-border divide-y">
                    {projection.attention.map((item) => (
                      <AttentionRow
                        key={`${item.resource}-${item.id}`}
                        item={item}
                        teamId={teamId}
                      />
                    ))}
                  </ul>
                ) : (
                  <div className="px-5 py-10 text-center">
                    <p className="text-text font-medium">{t("nothingNeedsAttention")}</p>
                    <p className="text-muted mt-1 text-sm">
                      {t("nothingNeedsAttentionDescription")}
                    </p>
                  </div>
                )}
              </section>

              <aside className="space-y-4" aria-label={t("overviewContext")}>
                <section className="surface rounded-md p-4" aria-labelledby="your-work-title">
                  <h2 id="your-work-title" className="text-text font-semibold">
                    {team?.role === "observer" ? t("operationalScope") : t("yourWork")}
                  </h2>
                  <p className="text-muted mt-1 text-sm">
                    {team?.role === "observer"
                      ? t("operationalScopeDescription")
                      : t("yourWorkDescription", { count: projection.counts.assignedToMe })}
                  </p>
                  {team?.role !== "observer" && projection.assignedIncidents.length > 0 ? (
                    <ul className="mt-3 space-y-2">
                      {projection.assignedIncidents.slice(0, 3).map((incident) => (
                        <li key={incident.id}>
                          <Link
                            href={teamPath(teamId, "incidents", incident.id)}
                            className="text-text hover:text-gold block truncate text-sm transition-colors"
                          >
                            {incident.title}
                          </Link>
                        </li>
                      ))}
                    </ul>
                  ) : null}
                </section>

                <section className="surface rounded-md p-4" aria-labelledby="blocked-work-title">
                  <h2 id="blocked-work-title" className="text-text font-semibold">
                    {t("blockedReleases")}
                  </h2>
                  {projection.blockedReleases.length > 0 ? (
                    <ul className="mt-3 space-y-3">
                      {projection.blockedReleases.slice(0, 3).map((release) => (
                        <li key={release.release_id}>
                          <Link
                            href={teamPath(teamId, "releases", release.release_id)}
                            className="text-text hover:text-gold block truncate text-sm font-medium transition-colors"
                          >
                            {release.title}
                          </Link>
                          <p className="text-muted mt-1 truncate text-xs">
                            {release.blockers[0]
                              ? t("blockedBy", { title: release.blockers[0].title })
                              : t("blockedReleasesDescription")}
                          </p>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="text-muted mt-2 text-sm">{t("noBlockedReleases")}</p>
                  )}
                </section>
              </aside>
            </div>
          </div>
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
