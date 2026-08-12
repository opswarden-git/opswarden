"use client";

import { useLocale, useTranslations } from "next-intl";
import { RunStatus } from "@/components/automations/RunsView";
import { SeverityChip } from "@/components/incidents/SeverityChip";
import { StateChip } from "@/components/incidents/StateChip";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { Alert } from "@/components/ui/Alert";
import { Link } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useAutomationRules, useAutomationRuns } from "@/lib/queries/automations";
import { useIncidentQueue } from "@/lib/queries/incidents";
import { useReleases } from "@/lib/queries/releases";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { cn, formatRelativeAge } from "@/lib/utils";

const previewLimit = 5;

function OverviewSection({
  children,
  count,
  href,
  title,
}: {
  children: React.ReactNode;
  count: number;
  href: string;
  title: string;
}) {
  return (
    <section aria-label={title} className="surface min-w-0 overflow-hidden rounded-md">
      <header className="border-border border-b px-4 py-3">
        <Link href={href} className="group inline-flex items-baseline gap-2">
          <h2 className="text-text group-hover:text-gold text-sm font-semibold transition-colors">
            {title}
          </h2>
          <span className="text-muted text-xs tabular-nums">{count}</span>
        </Link>
      </header>
      {children}
    </section>
  );
}

export function TeamOverviewPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Teams");
  const ta = useTranslations("Automations");
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
  const activeReleases = (releases.data ?? []).filter(
    (release) => release.state !== "completed" && release.state !== "cancelled",
  );
  const ruleNames = new Map((rules.data ?? []).map((rule) => [rule.id, rule.name]));
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
    <PageLayout>
      <PageContent
        state={state}
        loadingFallback={
          <div
            className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3"
            aria-label={t("loadingOverview")}
          >
            {Array.from({ length: 3 }, (_, index) => (
              <div key={index} className="surface h-72 animate-pulse rounded-md" />
            ))}
          </div>
        }
        errorFallback={<Alert tone="danger">{t("overviewUnavailable")}</Alert>}
      >
        <div
          className={cn(
            "grid items-start gap-4",
            canViewRuns ? "lg:grid-cols-2 xl:grid-cols-3" : "lg:grid-cols-2",
          )}
        >
          <OverviewSection
            title={t("overviewViews.incidents")}
            count={team?.active_incident_count ?? activeIncidents.length}
            href={teamPath(teamId, "incidents")}
          >
            {activeIncidents.length ? (
              <ul className="divide-border divide-y">
                {activeIncidents.slice(0, previewLimit).map((incident) => (
                  <li key={incident.id}>
                    <Link
                      href={teamPath(teamId, "incidents", incident.id)}
                      className="block px-4 py-3 transition-colors hover:bg-white/[0.04]"
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
                      <div className="mt-2 flex items-center gap-2">
                        <StateChip status={incident.status} />
                        <SeverityChip severity={incident.severity} />
                      </div>
                    </Link>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-muted px-4 py-8 text-center text-sm">
                {t("overviewEmpty.incidents")}
              </p>
            )}
          </OverviewSection>

          <OverviewSection
            title={t("overviewViews.releases")}
            count={team?.active_release_count ?? activeReleases.length}
            href={teamPath(teamId, "releases")}
          >
            {activeReleases.length ? (
              <ul className="divide-border divide-y">
                {activeReleases.slice(0, previewLimit).map((release) => (
                  <li key={release.release_id}>
                    <Link
                      href={teamPath(teamId, "releases", release.release_id)}
                      className="block px-4 py-3 transition-colors hover:bg-white/[0.04]"
                    >
                      <div className="flex min-w-0 items-start justify-between gap-3">
                        <span className="text-text truncate text-sm font-medium">
                          {release.title}
                        </span>
                        <time className="text-muted shrink-0 text-xs" dateTime={release.created_at}>
                          {formatRelativeAge(release.created_at, locale)}
                        </time>
                      </div>
                      <div className="mt-2 flex items-center justify-between gap-3">
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
              <p className="text-muted px-4 py-8 text-center text-sm">
                {t("overviewEmpty.releases")}
              </p>
            )}
          </OverviewSection>

          {canViewRuns ? (
            <OverviewSection
              title={t("overviewViews.runs")}
              count={runs.data?.length ?? 0}
              href={teamPath(teamId, "runs")}
            >
              {runs.data?.length ? (
                <ul className="divide-border divide-y">
                  {runs.data.slice(0, previewLimit).map((run) => (
                    <li key={run.id} className="px-4 py-3">
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
                      <div className="mt-2 flex items-center justify-between gap-3 text-xs">
                        <RunStatus status={run.status} />
                        {run.incident_id ? (
                          <Link
                            href={teamPath(teamId, "incidents", run.incident_id)}
                            className="text-gold hover:text-gold-hover"
                          >
                            {ta("openIncident")}
                          </Link>
                        ) : run.error_code ? (
                          <span className="text-sev-critical truncate" title={run.error_code}>
                            {run.error_code}
                          </span>
                        ) : null}
                      </div>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-muted px-4 py-8 text-center text-sm">
                  {t("overviewEmpty.runs")}
                </p>
              )}
            </OverviewSection>
          ) : null}
        </div>
      </PageContent>
    </PageLayout>
  );
}
