"use client";

import React from "react";
import { AlertCircle, Shield } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { CreateIncidentDialog } from "@/components/incidents/CreateIncidentDialog";
import { IncidentTable, IncidentTableSkeleton } from "@/components/incidents/IncidentTable";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { Button, buttonClassNames } from "@/components/ui/Button";
import {
  CollectionSearch,
  MobileCollectionFilters,
  TableFilterControl,
  TableSortControl,
} from "@/components/ui/CollectionControls";
import { Link, usePathname, useRouter } from "@/i18n/routing";
import { deriveCapabilities } from "@/lib/capabilities";
import {
  type IncidentSeverity,
  type IncidentStatus,
  useIncidentQueue,
} from "@/lib/queries/incidents";
import { useTeamMembers, useTeams } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";

type IncidentView = "all" | IncidentStatus;
type IncidentSort = "newest" | "oldest" | "severity";

const VIEWS: IncidentView[] = ["open", "acknowledged", "escalated", "resolved", "all"];
const SEVERITIES: IncidentSeverity[] = ["critical", "high", "medium", "low"];
const SORTS: IncidentSort[] = ["newest", "oldest", "severity"];

export function IncidentsPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Incidents");
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const searchParamsString = searchParams.toString();
  const { data: teams, isLoading: isLoadingTeams, error: teamsError } = useTeams();
  const { data: members } = useTeamMembers(teamId);
  const activeTeam = teams?.find((team) => team.team_id === teamId);
  const capabilities = deriveCapabilities(activeTeam?.role ?? "observer");
  const hasNoTeams = teams?.length === 0;

  const requestedView = searchParams.get("view") as IncidentView | null;
  const view = requestedView && VIEWS.includes(requestedView) ? requestedView : "open";
  const requestedSeverity = searchParams.get("severity") as IncidentSeverity | null;
  const severity =
    requestedSeverity && SEVERITIES.includes(requestedSeverity) ? requestedSeverity : undefined;
  const assignee = searchParams.get("assignee") || undefined;
  const urlQuery = searchParams.get("q") ?? "";
  const requestedSort = searchParams.get("sort") as IncidentSort | null;
  const sort = requestedSort && SORTS.includes(requestedSort) ? requestedSort : "newest";

  const {
    data: queue,
    isLoading: isLoadingIncidents,
    error,
  } = useIncidentQueue(teamId, {
    status: view === "all" ? undefined : view,
    severity,
    assignee,
    query: urlQuery || undefined,
    sort,
  });

  const commitSearch = React.useCallback(
    (value: string) => {
      const params = new URLSearchParams(searchParamsString);
      const normalized = value.trim();
      if (normalized) params.set("q", normalized);
      else params.delete("q");
      const suffix = params.toString();
      router.replace(suffix ? `${pathname}?${suffix}` : pathname);
    },
    [pathname, router, searchParamsString],
  );

  const paramsWith = (name: string, value?: string) => {
    const params = new URLSearchParams(searchParams.toString());
    if (value) params.set(name, value);
    else params.delete(name);
    const suffix = params.toString();
    return suffix ? `${pathname}?${suffix}` : pathname;
  };

  const setParam = (name: string, value?: string) => router.push(paramsWith(name, value));

  const counts = queue?.counts ?? {
    all: 0,
    open: 0,
    acknowledged: 0,
    escalated: 0,
    resolved: 0,
  };
  const incidents = queue?.items ?? [];
  const hasIncidents = counts.all > 0;
  const contentState: PageContentState =
    isLoadingTeams || isLoadingIncidents
      ? "loading"
      : teamsError || error
        ? "error"
        : hasNoTeams || incidents.length === 0
          ? "empty"
          : "ready";

  const clearFilters = () => router.push(`${pathname}?view=all`);
  const assignableMembers = (members ?? []).filter((member) => member.can_be_assigned_incident);
  const viewLabel = (value: IncidentView) => t(`view${value[0].toUpperCase()}${value.slice(1)}`);
  const severityLabel = (value: IncidentSeverity) =>
    t(`severity${value[0].toUpperCase()}${value.slice(1)}`);
  const activeFilterCount =
    (view === "all" ? 0 : 1) +
    (severity ? 1 : 0) +
    (assignee ? 1 : 0) +
    (sort === "newest" ? 0 : 1);
  const filterFieldClass = "space-y-2";
  const filterSelectClass = "ow-input h-10 w-full rounded-md px-3 text-sm";
  const filterFields = (
    <>
      <label className={filterFieldClass}>
        <span className="text-muted block text-sm uppercase">{t("colStatus")}</span>
        <select
          value={view}
          onChange={(event) => setParam("view", event.target.value)}
          className={filterSelectClass}
        >
          {VIEWS.map((value) => (
            <option key={value} value={value}>
              {viewLabel(value)} ({counts[value]})
            </option>
          ))}
        </select>
      </label>
      <label className={filterFieldClass}>
        <span className="text-muted block text-sm uppercase">{t("colSeverity")}</span>
        <select
          value={severity ?? ""}
          onChange={(event) => setParam("severity", event.target.value || undefined)}
          className={filterSelectClass}
        >
          <option value="">{t("allSeverities")}</option>
          {SEVERITIES.map((value) => (
            <option key={value} value={value}>
              {severityLabel(value)}
            </option>
          ))}
        </select>
      </label>
      <label className={filterFieldClass}>
        <span className="text-muted block text-sm uppercase">{t("colAssignee")}</span>
        <select
          value={assignee ?? ""}
          onChange={(event) => setParam("assignee", event.target.value || undefined)}
          className={filterSelectClass}
        >
          <option value="">{t("allAssignees")}</option>
          <option value="unassigned">{t("unassigned")}</option>
          {assignableMembers.map((member) => (
            <option key={member.user_id} value={member.user_id}>
              {member.email}
            </option>
          ))}
        </select>
      </label>
      <label className={filterFieldClass}>
        <span className="text-muted block text-sm uppercase">{t("sortLabel")}</span>
        <select
          value={sort}
          onChange={(event) => setParam("sort", event.target.value)}
          className={filterSelectClass}
        >
          {SORTS.map((value) => (
            <option key={value} value={value}>
              {t(`sort${value[0].toUpperCase()}${value.slice(1)}`)}
            </option>
          ))}
        </select>
      </label>
    </>
  );
  const headers = {
    colStatus: (
      <TableFilterControl
        label={t("colStatus")}
        value={view}
        activeLabel={viewLabel(view)}
        onChange={(value) => setParam("view", value)}
        options={VIEWS.map((value) => ({
          value,
          label: `${viewLabel(value)} (${counts[value]})`,
        }))}
      />
    ),
    colAssignee: (
      <TableFilterControl
        label={t("colAssignee")}
        value={assignee ?? ""}
        activeLabel={
          assignee === "unassigned"
            ? t("unassigned")
            : assignableMembers.find((member) => member.user_id === assignee)?.email
        }
        onChange={(value) => setParam("assignee", value || undefined)}
        options={[
          { value: "", label: t("allAssignees") },
          { value: "unassigned", label: t("unassigned") },
          ...assignableMembers.map((member) => ({ value: member.user_id, label: member.email })),
        ]}
      />
    ),
    colSeverity: (
      <TableFilterControl
        label={t("colSeverity")}
        value={severity ?? ""}
        activeLabel={severity ? severityLabel(severity) : undefined}
        onChange={(value) => setParam("severity", value || undefined)}
        options={[
          { value: "", label: t("allSeverities") },
          ...SEVERITIES.map((value) => ({ value, label: severityLabel(value) })),
        ]}
      />
    ),
    colAge: (
      <TableSortControl
        label={t("colAge")}
        direction={sort === "newest" ? "ascending" : sort === "oldest" ? "descending" : undefined}
        onToggle={() => setParam("sort", sort === "newest" ? "oldest" : "newest")}
      />
    ),
  };

  return (
    <PageLayout>
      <PageHeader
        actions={
          !hasNoTeams ? (
            <>
              <CollectionSearch
                key={urlQuery}
                initialValue={urlQuery}
                label={t("searchLabel")}
                placeholder={t("searchPlaceholder")}
                onCommit={commitSearch}
              />
              <MobileCollectionFilters
                activeCount={activeFilterCount}
                label={t("filtersLabel")}
                title={t("filtersLabel")}
                clearLabel={t("clearFilters")}
                closeLabel={t("close")}
                doneLabel={t("done")}
                onClear={clearFilters}
              >
                {filterFields}
              </MobileCollectionFilters>
              {capabilities.canCreateIncident ? <CreateIncidentDialog teamId={teamId} /> : null}
            </>
          ) : null
        }
      />

      <PageContent
        state={contentState}
        loadingFallback={<IncidentTableSkeleton />}
        errorFallback={<Alert tone="danger">{t("failedToLoad")}</Alert>}
        emptyFallback={
          hasNoTeams ? (
            <div className="surface rounded-md p-12 text-center">
              <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
              <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
              <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
              <Link href="/teams" className={buttonClassNames({ variant: "primary", size: "lg" })}>
                {t("goToTeams")}
              </Link>
            </div>
          ) : (
            <div className="surface rounded-md p-12 text-center">
              <AlertCircle className="text-muted/50 mx-auto mb-4 h-12 w-12" />
              <h3 className="text-text text-lg font-medium">
                {hasIncidents ? t("noMatchingIncidents") : t("noIncidentsYet")}
              </h3>
              <p className="text-muted mt-2 text-sm">
                {hasIncidents ? t("noMatchingIncidentsDesc") : t("noIncidentsDesc")}
              </p>
              {hasIncidents ? (
                <Button className="mt-6" onClick={clearFilters}>
                  {t("clearFilters")}
                </Button>
              ) : null}
            </div>
          )
        }
      >
        <IncidentTable
          incidents={incidents}
          headers={headers}
          ageSortDirection={
            sort === "newest" ? "ascending" : sort === "oldest" ? "descending" : undefined
          }
        />
      </PageContent>
    </PageLayout>
  );
}
