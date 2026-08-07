"use client";

import { useEffect, useState } from "react";
import { ArrowLeft, History, Plus } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { Button, buttonClassNames } from "@/components/ui/Button";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Link, useRouter } from "@/i18n/routing";
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
  const t = useTranslations("Automations");
  return (
    <div className="surface overflow-hidden rounded-md" aria-label={t("loading")}>
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
  const [isCreatingRule, setIsCreatingRule] = useState(false);
  const t = useTranslations("Automations");
  const router = useRouter();
  const searchParams = useSearchParams();
  const requestedView = searchParams.get("view");
  const view = automationView(requestedView);
  const teams = useTeams();
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canManage = team ? deriveCapabilities(team.role).canManageAutomations : false;
  const catalog = useAutomationCatalog(canManage);
  const connections = useTeamConnections(teamId, canManage);
  const rules = useAutomationRules(teamId, canManage);
  const runs = useAutomationRuns(teamId, canManage && view === "runs");

  const basePath = teamPath(teamId, "automations");

  useEffect(() => {
    if (requestedView && requestedView !== view) router.replace(basePath);
  }, [basePath, requestedView, router, view]);
  const isLoading =
    teams.isLoading ||
    (canManage &&
      (catalog.isLoading ||
        connections.isLoading ||
        rules.isLoading ||
        (view === "runs" && runs.isLoading)));
  const hasError =
    !!teams.error ||
    !team ||
    (canManage &&
      !!(catalog.error || connections.error || rules.error || (view === "runs" && runs.error)));
  const state: PageContentState = isLoading ? "loading" : hasError ? "error" : "ready";

  return (
    <PageLayout>
      <PageHeader
        actions={
          canManage ? (
            view === "rules" ? (
              <>
                <Link
                  href={`${basePath}?view=runs`}
                  className={buttonClassNames({ variant: "secondary" })}
                >
                  <History className="h-4 w-4" aria-hidden="true" /> {t("runHistory")}
                </Link>
                <Button variant="primary" onClick={() => setIsCreatingRule(true)}>
                  <Plus className="h-4 w-4" aria-hidden="true" /> {t("newRule")}
                </Button>
              </>
            ) : view === "runs" ? (
              <Link href={basePath} className={buttonClassNames({ variant: "secondary" })}>
                <ArrowLeft className="h-4 w-4" aria-hidden="true" /> {t("backToRules")}
              </Link>
            ) : undefined
          ) : undefined
        }
      />
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
          <div>
            {view === "rules" ? (
              <RulesView
                teamId={teamId}
                catalog={catalog.data ?? []}
                connections={connections.data ?? []}
                rules={rules.data ?? []}
                isCreatingRule={isCreatingRule}
                setIsCreatingRule={setIsCreatingRule}
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
