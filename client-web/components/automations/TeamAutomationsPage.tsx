"use client";

import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { PageTabs } from "@/components/layout/PageTabs";
import { TeamHeader } from "@/components/teams/TeamHeader";
import { automationView } from "@/lib/automation-routing";
import { deriveCapabilities } from "@/lib/capabilities";
import {
  useAutomationCatalog,
  useAutomationRules,
  useAutomationRuns,
  useTeamConnections,
} from "@/lib/queries/automations";
import { useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { ConnectionsView } from "./ConnectionsView";
import { RulesView } from "./RulesView";
import { RunsView } from "./RunsView";

function AutomationLoading() {
  return (
    <div className="surface overflow-hidden rounded-md" aria-label="Loading automations">
      <div className="surface-subtle border-border h-11 border-b" />
      <div className="divide-border divide-y">
        {Array.from({ length: 4 }, (_, index) => (
          <div key={index} className="flex h-16 animate-pulse items-center gap-8 px-5">
            <span className="bg-panel-2 h-4 w-1/4 rounded" />
            <span className="bg-panel-2 h-4 w-1/5 rounded" />
            <span className="bg-panel-2 h-4 w-1/5 rounded" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function TeamAutomationsPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Automations");
  const searchParams = useSearchParams();
  const view = automationView(searchParams.get("view"));
  const teams = useTeams();
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canManage = team ? deriveCapabilities(team.role).canManageAutomations : false;
  const catalog = useAutomationCatalog(canManage);
  const connections = useTeamConnections(teamId, canManage);
  const rules = useAutomationRules(teamId, canManage);
  const runs = useAutomationRuns(teamId, canManage && view === "runs");

  const basePath = teamPath(teamId, "automations");
  const isLoading =
    teams.isLoading ||
    (canManage && (catalog.isLoading || connections.isLoading || rules.isLoading)) ||
    (canManage && view === "runs" && runs.isLoading);
  const hasError =
    !!teams.error ||
    !team ||
    (canManage && !!(catalog.error || connections.error || rules.error)) ||
    (canManage && view === "runs" && !!runs.error);
  const state: PageContentState = isLoading ? "loading" : hasError ? "error" : "ready";

  return (
    <PageLayout>
      {team ? <TeamHeader team={team} /> : null}
      <PageContent
        state={state}
        loadingFallback={<AutomationLoading />}
        errorFallback={<Alert tone="danger">{t("unavailable")}</Alert>}
      >
        {team && !canManage ? (
          <Alert tone="warning" title={t("managerOnlyTitle")}>
            {t("managerOnlyDescription")}
          </Alert>
        ) : team && canManage ? (
          <div className="space-y-6">
            <header>
              <h2 className="text-text text-xl font-semibold">{t("title")}</h2>
              <p className="text-muted mt-1 max-w-3xl text-sm">{t("description")}</p>
            </header>
            <PageTabs
              ariaLabel={t("viewsLabel")}
              tabs={[
                {
                  href: `${basePath}?view=rules`,
                  label: t("rules"),
                  count: rules.data?.length ?? 0,
                  active: view === "rules",
                },
                {
                  href: `${basePath}?view=connections`,
                  label: t("connections"),
                  count: connections.data?.length ?? 0,
                  active: view === "connections",
                },
                {
                  href: `${basePath}?view=runs`,
                  label: t("runs"),
                  count: runs.data?.length,
                  active: view === "runs",
                },
              ]}
            />

            {view === "rules" ? (
              <RulesView
                teamId={teamId}
                catalog={catalog.data ?? []}
                connections={connections.data ?? []}
                rules={rules.data ?? []}
              />
            ) : null}
            {view === "connections" ? (
              <ConnectionsView
                teamId={teamId}
                catalog={catalog.data ?? []}
                connections={connections.data ?? []}
                rules={rules.data ?? []}
              />
            ) : null}
            {view === "runs" ? (
              <RunsView
                teamId={teamId}
                runs={runs.data ?? []}
                rules={rules.data ?? []}
                isFetching={runs.isFetching}
                onRefresh={() => runs.refetch()}
              />
            ) : null}
          </div>
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
