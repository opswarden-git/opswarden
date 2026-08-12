"use client";

import { useState } from "react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { MobileCollectionFilters } from "@/components/ui/CollectionControls";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { useRouter } from "@/i18n/routing";
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

export function TeamAutomationsPage({
  teamId,
  resource = "rules",
}: {
  teamId: string;
  resource?: "rules" | "integrations" | "runs";
}) {
  const [isCreatingRule, setIsCreatingRule] = useState(false);
  const t = useTranslations("Automations");
  const router = useRouter();
  const searchParams = useSearchParams();
  const view = resource === "integrations" ? "connections" : resource;
  const teams = useTeams();
  const team = teams.data?.find((candidate) => candidate.team_id === teamId);
  const canManage = team ? deriveCapabilities(team.role).canManageAutomations : false;
  const needsConfiguration = canManage && resource !== "runs";
  const catalog = useAutomationCatalog(needsConfiguration);
  const connections = useTeamConnections(teamId, needsConfiguration);
  const rules = useAutomationRules(teamId, canManage);
  const runs = useAutomationRuns(teamId, canManage && resource === "runs");

  const basePath = teamPath(teamId, resource);
  const ruleStatus = ["enabled", "disabled"].includes(searchParams.get("status") ?? "")
    ? (searchParams.get("status") as "enabled" | "disabled")
    : "all";
  const ruleSort = ["next_asc", "next_desc", "updated_asc", "updated_desc"].includes(
    searchParams.get("sort") ?? "",
  )
    ? (searchParams.get("sort") as "next_asc" | "next_desc" | "updated_asc" | "updated_desc")
    : "updated_desc";
  const runStatus = searchParams.get("status") ?? "all";
  const runRule = searchParams.get("rule") ?? "all";
  const runSort = ["duration_asc", "duration_desc", "started_asc", "started_desc"].includes(
    searchParams.get("sort") ?? "",
  )
    ? (searchParams.get("sort") as
        "duration_asc" | "duration_desc" | "started_asc" | "started_desc")
    : "started_desc";
  const setParam = (name: string, value?: string) => {
    const params = new URLSearchParams(searchParams.toString());
    if (value) params.set(name, value);
    else params.delete(name);
    const suffix = params.toString();
    router.push(suffix ? `${basePath}?${suffix}` : basePath);
  };
  const clearCollectionFilters = () => {
    router.push(basePath);
  };
  const filterSelectClass = "ow-input h-10 w-full rounded-md px-3 text-sm";
  const ruleFilterFields = (
    <>
      <label className="space-y-2">
        <span className="text-muted block text-sm uppercase">{t("colStatus")}</span>
        <select
          value={ruleStatus}
          onChange={(event) =>
            setParam("status", event.target.value === "all" ? undefined : event.target.value)
          }
          className={filterSelectClass}
        >
          <option value="all">{t("allStatuses")}</option>
          <option value="enabled">{t("enabled")}</option>
          <option value="disabled">{t("disabled")}</option>
        </select>
      </label>
      <label className="space-y-2">
        <span className="text-muted block text-sm uppercase">{t("sortLabel")}</span>
        <select
          value={ruleSort}
          onChange={(event) => setParam("sort", event.target.value)}
          className={filterSelectClass}
        >
          <option value="updated_desc">{t("sortUpdatedNewest")}</option>
          <option value="updated_asc">{t("sortUpdatedOldest")}</option>
          <option value="next_asc">{t("sortNextSoonest")}</option>
          <option value="next_desc">{t("sortNextLatest")}</option>
        </select>
      </label>
    </>
  );
  const runStatuses = Array.from(new Set((runs.data ?? []).map((run) => run.status))).sort();
  const runFilterFields = (
    <>
      <label className="space-y-2">
        <span className="text-muted block text-sm uppercase">{t("colRule")}</span>
        <select
          value={runRule}
          onChange={(event) =>
            setParam("rule", event.target.value === "all" ? undefined : event.target.value)
          }
          className={filterSelectClass}
        >
          <option value="all">{t("allRules")}</option>
          {(rules.data ?? []).map((rule) => (
            <option key={rule.id} value={rule.id}>
              {rule.name}
            </option>
          ))}
        </select>
      </label>
      <label className="space-y-2">
        <span className="text-muted block text-sm uppercase">{t("colStatus")}</span>
        <select
          value={runStatus}
          onChange={(event) =>
            setParam("status", event.target.value === "all" ? undefined : event.target.value)
          }
          className={filterSelectClass}
        >
          <option value="all">{t("allStatuses")}</option>
          {runStatuses.map((status) => (
            <option key={status} value={status}>
              {status}
            </option>
          ))}
        </select>
      </label>
      <label className="space-y-2">
        <span className="text-muted block text-sm uppercase">{t("sortLabel")}</span>
        <select
          value={runSort}
          onChange={(event) => setParam("sort", event.target.value)}
          className={filterSelectClass}
        >
          <option value="started_desc">{t("sortStartedNewest")}</option>
          <option value="started_asc">{t("sortStartedOldest")}</option>
          <option value="duration_asc">{t("sortDurationShortest")}</option>
          <option value="duration_desc">{t("sortDurationLongest")}</option>
        </select>
      </label>
    </>
  );

  const isLoading =
    teams.isLoading ||
    (canManage &&
      (rules.isLoading ||
        (resource !== "runs" && (catalog.isLoading || connections.isLoading)) ||
        (resource === "runs" && runs.isLoading)));
  const hasError =
    !!teams.error ||
    !team ||
    (canManage &&
      !!(
        rules.error ||
        (resource !== "runs" && (catalog.error || connections.error)) ||
        (resource === "runs" && runs.error)
      ));
  const state: PageContentState = isLoading ? "loading" : hasError ? "error" : "ready";

  return (
    <PageLayout>
      <PageHeader
        actions={
          canManage ? (
            view === "rules" ? (
              <>
                <MobileCollectionFilters
                  activeCount={
                    (ruleStatus === "all" ? 0 : 1) + (ruleSort === "updated_desc" ? 0 : 1)
                  }
                  label={t("filtersLabel")}
                  title={t("filtersLabel")}
                  description={t("filtersDescription")}
                  clearLabel={t("clearFilters")}
                  closeLabel={t("filtersClose")}
                  doneLabel={t("done")}
                  onClear={clearCollectionFilters}
                >
                  {ruleFilterFields}
                </MobileCollectionFilters>
                <Button variant="primary" onClick={() => setIsCreatingRule(true)}>
                  {t("newRule")}
                </Button>
              </>
            ) : view === "runs" ? (
              <>
                <MobileCollectionFilters
                  activeCount={
                    (runStatus === "all" ? 0 : 1) +
                    (runRule === "all" ? 0 : 1) +
                    (runSort === "started_desc" ? 0 : 1)
                  }
                  label={t("filtersLabel")}
                  title={t("filtersLabel")}
                  description={t("filtersDescription")}
                  clearLabel={t("clearFilters")}
                  closeLabel={t("filtersClose")}
                  doneLabel={t("done")}
                  onClear={clearCollectionFilters}
                >
                  {runFilterFields}
                </MobileCollectionFilters>
                <Button size="sm" onClick={() => runs.refetch()} loading={runs.isFetching}>
                  {t("refresh")}
                </Button>
              </>
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
                statusFilter={ruleStatus}
                sort={ruleSort}
                onStatusFilterChange={(status) =>
                  setParam("status", status === "all" ? undefined : status)
                }
                onSortChange={(nextSort) =>
                  setParam("sort", nextSort === "updated_desc" ? undefined : nextSort)
                }
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
                statusFilter={runStatus}
                ruleFilter={runRule}
                sort={runSort}
                showControls
                onStatusFilterChange={(status) =>
                  setParam("status", status === "all" ? undefined : status)
                }
                onRuleFilterChange={(rule) => setParam("rule", rule === "all" ? undefined : rule)}
                onSortChange={(nextSort) =>
                  setParam("sort", nextSort === "started_desc" ? undefined : nextSort)
                }
              />
            ) : null}
          </div>
        ) : null}
      </PageContent>
    </PageLayout>
  );
}
