"use client";

import { useTranslations } from "next-intl";
import { RunsView } from "@/components/automations/RunsView";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { Link } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { useAutomationRules, useAutomationRuns } from "@/lib/queries/automations";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";

/**
 * What automation has actually done to this Team: what fired, what it created,
 * what failed.
 *
 * The Action->REAction engine is a Phase 2 Core deliverable that was only ever
 * visible from inside a configuration screen, one tab deep. The data was
 * already served; this gives it an operational home of its own.
 *
 * Listing runs is Manager-only on the server (`require_manager`), so the
 * navigation entry is gated on the same capability rather than leading the
 * other roles to a wall.
 */
export function TeamActivityPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Activity");
  const teams = useTeams();
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canManage = team ? deriveCapabilities(team.role).canManageAutomations : false;
  const rules = useAutomationRules(teamId, canManage);
  const runs = useAutomationRuns(teamId, canManage);

  const isLoading = teams.isLoading || (canManage && (rules.isLoading || runs.isLoading));
  const hasError = !!teams.error || !team || (canManage && !!(rules.error || runs.error));
  const state: PageContentState = isLoading ? "loading" : hasError ? "error" : "ready";

  return (
    <PageLayout>
      {/*
       * The operational header, not TeamHeader: that one carries the Team
       * configuration tab strip (Overview · Members · Automations · Settings),
       * and Activity is not one of those. Incidents and Releases use this same
       * shape -- a link back to the Team, then the surface's own title.
       */}
      <PageHeader
        context={
          team ? (
            <Link
              href={teamPath(teamId, "overview")}
              className="hover:text-text transition-colors hover:underline"
            >
              {team.name}
            </Link>
          ) : null
        }
        title={t("title")}
        description={t("description")}
      />
      <PageContent
        state={state}
        loadingFallback={
          <div className="surface h-64 animate-pulse rounded-md" aria-label={t("loading")} />
        }
        errorFallback={<Alert tone="danger">{t("unavailable")}</Alert>}
      >
        {team && !canManage ? (
          <Alert tone="warning" title={t("managerOnlyTitle")}>
            {t("managerOnlyDescription")}
          </Alert>
        ) : team ? (
          <div className="space-y-6">
            <RunsView
              teamId={teamId}
              runs={runs.data ?? []}
              rules={rules.data ?? []}
              isFetching={runs.isFetching}
              onRefresh={() => runs.refetch()}
            />
          </div>
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
