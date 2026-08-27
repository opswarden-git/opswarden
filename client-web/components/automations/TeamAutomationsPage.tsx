"use client";

import { useState } from "react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { Skeleton } from "@/components/ui/Skeleton";
import { MobileCollectionFilters } from "@/components/ui/CollectionControls";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "@/components/ui/OperationalTable";
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
import { cn } from "@/lib/utils";
import { ConnectionsView } from "./ConnectionsView";
import { RulesView } from "./RulesView";
import { RunsView } from "./RunsView";

function AutomationTableLoading({
  columns,
  label,
  rows = 5,
}: {
  columns: number;
  label: string;
  rows?: number;
}) {
  const widths =
    columns === 7
      ? ["w-32", "w-20", "w-28", "w-28", "w-28", "w-20", "w-8"]
      : ["w-16", "w-20", "w-28", "w-20", "w-28", "w-16"];
  return (
    <OperationalTable label={label} className="min-w-[820px]" containerClassName="overflow-x-auto">
      <OperationalTableHead>
        <tr>
          {widths.map((width, index) => (
            <OperationalTableHeaderCell key={index}>
              <Skeleton className={cn("h-3", width)} />
            </OperationalTableHeaderCell>
          ))}
        </tr>
      </OperationalTableHead>
      <OperationalTableBody>
        {Array.from({ length: rows }, (_, rowIndex) => (
          <OperationalTableRow key={rowIndex} className="hover:bg-transparent">
            <OperationalTableRowHeader>
              <Skeleton className={cn("h-4", widths[0])} />
            </OperationalTableRowHeader>
            {widths.slice(1).map((width, columnIndex) => (
              <OperationalTableCell key={columnIndex}>
                <Skeleton className={cn(columnIndex === 0 ? "h-5 rounded-full" : "h-4", width)} />
              </OperationalTableCell>
            ))}
          </OperationalTableRow>
        ))}
      </OperationalTableBody>
    </OperationalTable>
  );
}

function AutomationLoading({ view }: { view: "connections" | "rules" | "runs" }) {
  const t = useTranslations("Automations");
  if (view === "connections") {
    return (
      <div className="space-y-8" aria-label={t("loading")} aria-busy="true">
        {[2, 4].map((rows, groupIndex) => (
          <section key={groupIndex}>
            <div className="mb-2 flex items-center gap-2 px-1">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-4" />
            </div>
            <div className="surface divide-border-muted divide-y overflow-hidden rounded-md">
              {Array.from({ length: rows }, (_, index) => (
                <div key={index} className="flex min-h-16 items-center gap-4 px-4 py-3">
                  <Skeleton className="h-8 w-8 shrink-0" />
                  <Skeleton className="h-4 w-32" />
                  {groupIndex === 0 ? <Skeleton className="ml-auto h-5 w-20 rounded-full" /> : null}
                  <Skeleton className={cn("h-8 w-20", groupIndex !== 0 && "ml-auto")} />
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>
    );
  }

  if (view === "rules") {
    return (
      <div aria-label={t("loading")} aria-busy="true">
        <div className="hidden overflow-x-auto lg:block">
          <AutomationTableLoading columns={7} label={t("loading")} rows={4} />
        </div>
        <div className="surface divide-border-muted divide-y overflow-hidden rounded-md lg:hidden">
          {Array.from({ length: 4 }, (_, index) => (
            <div key={index} className="space-y-3 p-4">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1 space-y-2">
                  <Skeleton className="h-4 w-2/3" />
                  <Skeleton className="h-5 w-20 rounded-full" />
                </div>
                <Skeleton className="h-8 w-8" />
              </div>
              <div className="surface-subtle border-border space-y-2 rounded border px-3 py-2">
                {[0, 1, 2, 3].map((row) => (
                  <div key={row} className="flex items-center justify-between gap-4">
                    <Skeleton className="h-3 w-16" />
                    <Skeleton className="h-3 w-28" />
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto" aria-label={t("loading")} aria-busy="true">
      <AutomationTableLoading columns={6} label={t("loading")} />
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
        loadingFallback={<AutomationLoading view={view} />}
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
