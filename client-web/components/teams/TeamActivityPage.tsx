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

/** Gives automation runs an operational home outside their configuration page. */
export function TeamActivityPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Activity");
  const teams = useTeams();
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canManage = team ? deriveCapabilities(team.role).canManageAutomations : false;
  const rules = useAutomationRules(teamId, canManage);
  const runs = useAutomationRuns(teamId, canManage);

  const isLoading = teams.isLoading || (canManage && (rules.isLoading || runs.isLoading));
  const hasError =
    !!teams.error || (!teams.isLoading && !team) || !!(canManage && (rules.error || runs.error));
  const state: PageContentState = isLoading ? "loading" : hasError ? "error" : "ready";

  return (
    <PageLayout>
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
          <RunsView
            teamId={teamId}
            runs={runs.data ?? []}
            rules={rules.data ?? []}
            isFetching={runs.isFetching}
            onRefresh={() => runs.refetch()}
          />
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
