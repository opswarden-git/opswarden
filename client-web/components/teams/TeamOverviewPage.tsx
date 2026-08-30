"use client";

import { useLocale, useTranslations } from "next-intl";
import { RunStatus } from "@/components/automations/RunsView";
import { useErrorText } from "@/lib/useErrorText";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { isOngoingRelease } from "@/components/releases/release-views";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { Alert } from "@/components/ui/Alert";
import { Skeleton } from "@/components/ui/Skeleton";
import { Link } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useAutomationRules, useAutomationRuns } from "@/lib/queries/automations";
import { useIncidentQueue } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { cn, formatRelativeAge } from "@/lib/utils";
import { OperationsCalendar, type OperationsCalendarEvent } from "./OperationsCalendar";

const previewLimit = 2;

function OverviewSkeleton({ canViewRuns, label }: { canViewRuns: boolean; label: string }) {
  const sectionCount = canViewRuns ? 3 : 2;
  return (
    <div
      aria-busy="true"
      aria-label={label}
      className="space-y-3 md:flex md:h-full md:min-h-0 md:flex-col md:space-y-0"
    >
      <div
        className="surface flex min-h-72 flex-col overflow-hidden rounded-md md:min-h-0 md:flex-1"
        data-skeleton-region="calendar"
      >
        <div className="border-border flex items-center gap-3 border-b px-4 py-2">
          <Skeleton className="mr-auto h-5 w-48" />
          <div className="flex gap-2">
            <Skeleton className="h-5 w-10" />
            <Skeleton className="h-5 w-12" />
          </div>
          <div className="flex gap-1">
            <Skeleton className="h-8 w-8 rounded-md" />
            <Skeleton className="h-8 w-8 rounded-md" />
          </div>
        </div>
        <div className="min-h-0 overflow-x-auto md:flex-1">
          <div className="min-w-[760px] md:flex md:h-full md:flex-col">
            <div className="border-border grid grid-cols-7 border-b">
              {Array.from({ length: 7 }, (_, index) => (
                <div key={index} className="flex justify-center px-2 py-2">
                  <Skeleton className="h-3 w-8" />
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7 md:min-h-0 md:flex-1 md:grid-rows-6">
              {Array.from({ length: 42 }, (_, index) => (
                <div
                  key={index}
                  className="border-border min-h-28 border-r border-b p-1.5 last:border-r-0 md:min-h-0"
                  data-skeleton-calendar-day="true"
                >
                  <div className="mb-1 flex h-6 items-center justify-end">
                    <Skeleton className="h-3 w-5" />
                  </div>
                  {index % 7 === 2 || index % 11 === 0 ? (
                    <Skeleton className="h-5 w-full rounded" />
                  ) : null}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
      <div
        className={cn(
          "grid gap-3 md:mt-3 md:shrink-0",
          canViewRuns ? "md:grid-cols-3" : "md:grid-cols-2",
        )}
      >
        {Array.from({ length: sectionCount }, (_, index) => (
          <div
            key={index}
            className="surface min-w-0 overflow-hidden rounded-md"
            data-skeleton-region="overview-summary"
          >
            <div className="border-border flex items-center justify-between border-b px-4 py-2">
              <div className="flex items-center gap-2">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-3 w-4" />
              </div>
              <Skeleton className="h-3 w-10" />
            </div>
            {[0, 1].map((row) => (
              <div key={row} className="border-border border-b px-4 py-2 last:border-b-0">
                <div className="flex items-center justify-between gap-3">
                  <Skeleton className="h-4 w-2/3" />
                  <Skeleton className="h-3 w-12" />
                </div>
                <div className="mt-1.5 flex items-center justify-between gap-2">
                  <Skeleton className="h-5 w-20 rounded-full" />
                  <Skeleton className="h-4 w-12" />
                </div>
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function OverviewSection({
  children,
  count,
  href,
  seeAllLabel,
  title,
}: {
  children: React.ReactNode;
  count: number;
  href: string;
  seeAllLabel: string;
  title: string;
}) {
  return (
    <section aria-label={title} className="surface min-w-0 overflow-hidden rounded-md">
      <header className="border-border flex items-center justify-between gap-3 border-b px-4 py-2">
        <div className="inline-flex min-w-0 items-baseline gap-2">
          <h2 className="text-text truncate text-sm font-semibold">{title}</h2>
          <span className="text-muted text-xs tabular-nums">{count}</span>
        </div>
        <Link
          href={href}
          aria-label={`${seeAllLabel}: ${title}`}
          className="text-muted hover:text-gold shrink-0 text-xs font-medium transition-colors"
        >
          {seeAllLabel}
        </Link>
      </header>
      {children}
    </section>
  );
}

export function TeamOverviewPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const ta = useTranslations("Automations");
  const errorText = useErrorText();
  const locale = useLocale();
  const teams = useTeams();
  const incidents = useIncidentQueue(teamId, { sort: "severity" });
  const releases = useReleases(teamId);
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canViewRuns = deriveCapabilities(team?.role ?? "observer").canManageAutomations;
  const rules = useAutomationRules(teamId, canViewRuns);
  const runs = useAutomationRuns(teamId, canViewRuns);

  const activeIncidents = (incidents.data?.items ?? []).filter(
    (incident) => incident.status !== "resolved",
  );
  const activeReleases = (releases.data ?? []).filter(isOngoingRelease);
  const ruleNames = new Map((rules.data ?? []).map((rule) => [rule.id, rule.name]));
  const calendarEvents: OperationsCalendarEvent[] = [
    ...(incidents.data?.items ?? []).map((incident) => ({
      id: incident.id,
      occurredAt: incident.created_at,
      href: teamPath(teamId, "incidents", incident.id),
      title: incident.title,
      type: "incident" as const,
    })),
    ...(releases.data ?? []).map((release) => ({
      id: release.release_id,
      occurredAt: release.created_at,
      href: teamPath(teamId, "releases", release.release_id),
      title: release.title,
      type: "release" as const,
    })),
    ...(canViewRuns ? (runs.data ?? []) : []).map((run) => ({
      id: run.id,
      occurredAt: run.started_at,
      endedAt: run.finished_at,
      href: teamPath(teamId, "runs"),
      title: run.rule_id ? (ruleNames.get(run.rule_id) ?? ta("deletedRule")) : ta("noRule"),
      type: "run" as const,
    })),
  ];
  const state: PageContentState =
    teams.isLoading ||
    incidents.isLoading ||
    releases.isLoading ||
    (canViewRuns && (rules.isLoading || runs.isLoading))
      ? "loading"
      : teams.error ||
          incidents.error ||
          releases.error ||
          (canViewRuns && (rules.error || runs.error)) ||
          !team
        ? "error"
        : "ready";

  return (
    <PageLayout fill className="gap-3 md:overflow-hidden md:pb-4">
      <PageContent
        className="md:min-h-0 md:flex-1"
        state={state}
        loadingFallback={
          <OverviewSkeleton canViewRuns={canViewRuns} label={t("loadingOverview")} />
        }
        errorFallback={<Alert tone="danger">{t("overviewUnavailable")}</Alert>}
      >
        <div className="space-y-3 md:flex md:h-full md:min-h-0 md:flex-col md:space-y-0">
          <OperationsCalendar
            className="md:min-h-0 md:flex-1"
            events={calendarEvents}
            locale={locale}
            labels={{
              calendar: t("calendar.label"),
              today: t("calendar.today"),
              previousMonth: t("calendar.previousMonth"),
              nextMonth: t("calendar.nextMonth"),
              previousWeek: t("calendar.previousWeek"),
              nextWeek: t("calendar.nextWeek"),
              month: t("calendar.month"),
              week: t("calendar.week"),
              incident: t("calendar.incident"),
              less: t("calendar.less"),
              more: (count) => t("calendar.more", { count }),
              release: t("calendar.release"),
              run: t("calendar.run"),
            }}
          />
          <div
            className={cn(
              "grid items-start gap-3 md:mt-3 md:shrink-0",
              canViewRuns ? "md:grid-cols-3" : "md:grid-cols-2",
            )}
          >
            <OverviewSection
              title={t("overviewViews.incidents")}
              count={team?.active_incident_count ?? activeIncidents.length}
              href={teamPath(teamId, "incidents")}
              seeAllLabel={t("overviewViews.seeAll")}
            >
              {activeIncidents.length ? (
                <ul className="divide-border-muted divide-y">
                  {activeIncidents.slice(0, previewLimit).map((incident) => (
                    <li key={incident.id}>
                      <Link
                        href={teamPath(teamId, "incidents", incident.id)}
                        className="block px-4 py-2 transition-colors hover:bg-white/[0.04]"
                      >
                        <div className="flex min-w-0 items-start justify-between gap-3">
                          <span className="text-text truncate text-sm font-medium">
                            {incident.title}
                          </span>
                          <time
                            className="text-muted shrink-0 text-xs"
                            dateTime={incident.created_at}
                          >
                            {formatRelativeAge(incident.created_at, locale)}
                          </time>
                        </div>
                        <div className="mt-1.5 flex items-center gap-2">
                          <StateChip status={incident.status} />
                          <SeverityChip severity={incident.severity} />
                        </div>
                      </Link>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted px-4 py-6 text-center text-sm">
                  {t("overviewEmpty.incidents")}
                </p>
              )}
            </OverviewSection>

            <OverviewSection
              title={t("overviewViews.releases")}
              count={team?.active_release_count ?? activeReleases.length}
              href={teamPath(teamId, "releases")}
              seeAllLabel={t("overviewViews.seeAll")}
            >
              {activeReleases.length ? (
                <ul className="divide-border-muted divide-y">
                  {activeReleases.slice(0, previewLimit).map((release) => (
                    <li key={release.release_id}>
                      <Link
                        href={teamPath(teamId, "releases", release.release_id)}
                        className="block px-4 py-2 transition-colors hover:bg-white/[0.04]"
                      >
                        <div className="flex min-w-0 items-start justify-between gap-3">
                          <span className="text-text truncate text-sm font-medium">
                            {release.title}
                          </span>
                          <time
                            className="text-muted shrink-0 text-xs"
                            dateTime={release.created_at}
                          >
                            {formatRelativeAge(release.created_at, locale)}
                          </time>
                        </div>
                        <div className="mt-1.5 flex items-center justify-between gap-3">
                          <ReleaseStateChip state={release.state} />
                          <span className="text-muted text-xs tabular-nums">
                            {release.progress.completed}/{release.progress.total}
                          </span>
                        </div>
                      </Link>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted px-4 py-6 text-center text-sm">
                  {t("overviewEmpty.releases")}
                </p>
              )}
            </OverviewSection>

            {canViewRuns ? (
              <OverviewSection
                title={t("overviewViews.runs")}
                count={runs.data?.length ?? 0}
                href={teamPath(teamId, "runs")}
                seeAllLabel={t("overviewViews.seeAll")}
              >
                {runs.data?.length ? (
                  <ul className="divide-border-muted divide-y">
                    {runs.data.slice(0, previewLimit).map((run) => (
                      <li key={run.id} className="px-4 py-2">
                        <div className="flex min-w-0 items-start justify-between gap-3">
                          <span className="text-text truncate text-sm font-medium">
                            {run.rule_id
                              ? (ruleNames.get(run.rule_id) ?? ta("deletedRule"))
                              : ta("noRule")}
                          </span>
                          <time className="text-muted shrink-0 text-xs" dateTime={run.started_at}>
                            {formatRelativeAge(run.started_at, locale)}
                          </time>
                        </div>
                        <div className="mt-1.5 flex items-center justify-between gap-3 text-xs">
                          <RunStatus status={run.status} />
                          {run.incident_id ? (
                            <Link
                              href={teamPath(teamId, "incidents", run.incident_id)}
                              className="text-gold hover:text-gold-hover"
                            >
                              {ta("openIncident")}
                            </Link>
                          ) : run.error_code ? (
                            <span
                              className="text-sev-critical truncate"
                              title={errorText(run.error_code)}
                            >
                              {errorText(run.error_code)}
                            </span>
                          ) : null}
                        </div>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-muted px-4 py-6 text-center text-sm">
                    {t("overviewEmpty.runs")}
                  </p>
                )}
              </OverviewSection>
            ) : null}
          </div>
        </div>
      </PageContent>
    </PageLayout>
  );
}
