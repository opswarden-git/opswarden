"use client";

import React from "react";
import { AlertCircle, Search, Shield } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useTranslations } from "next-intl";
import { CreateIncidentDialog } from "@/components/incidents/CreateIncidentDialog";
import { IncidentTable, IncidentTableSkeleton } from "@/components/incidents/IncidentTable";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { PageTabs } from "@/components/layout/PageTabs";
import { PageToolbar } from "@/components/layout/PageToolbar";
import { Alert } from "@/components/ui/Alert";
import { Button, buttonClassNames } from "@/components/ui/Button";
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

function IncidentSearch({
  initialValue,
  label,
  onCommit,
  placeholder,
}: {
  initialValue: string;
  label: string;
  onCommit: (value: string) => void;
  placeholder: string;
}) {
  const [value, setValue] = React.useState(initialValue);

  React.useEffect(() => {
    if (value === initialValue) return;
    const timer = window.setTimeout(() => onCommit(value), 250);
    return () => window.clearTimeout(timer);
  }, [initialValue, onCommit, value]);

  return (
    <label className="relative min-w-0 flex-1">
      <span className="sr-only">{label}</span>
      <Search
        className="text-muted pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2"
        aria-hidden="true"
      />
      <input
        type="search"
        value={value}
        onChange={(event) => setValue(event.target.value)}
        placeholder={placeholder}
        className="border-border bg-panel text-text placeholder:text-muted-2 focus:border-gold h-9 w-full rounded-md border pr-3 pl-9 text-sm outline-none"
      />
    </label>
  );
}

export function IncidentsPage({ teamId }: { teamId: string }) {
  const t = useTranslations("Incidents");
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const searchParamsString = searchParams.toString();
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
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
      : error
        ? "error"
        : hasNoTeams || incidents.length === 0
          ? "empty"
          : "ready";

  const tabHref = (nextView: IncidentView) => {
    const params = new URLSearchParams(searchParams.toString());
    if (nextView === "open") params.delete("view");
    else params.set("view", nextView);
    const suffix = params.toString();
    return suffix ? `${pathname}?${suffix}` : pathname;
  };

  const clearFilters = () => router.push(`${pathname}?view=all`);
  const assignableMembers = (members ?? []).filter(
    (member) => member.role === "manager" || member.role === "responder",
  );

  return (
    <PageLayout>
      <PageHeader
        context={
          isLoadingTeams ? (
            <span className="bg-muted/20 inline-block h-4 w-24 animate-pulse rounded" />
          ) : activeTeam ? (
            <Link
              href={teamPath(teamId, "overview")}
              className="hover:text-text hover:underline transition-colors"
            >
              {activeTeam.name}
            </Link>
          ) : null
        }
        title={t("title")}
        description={t("queueDescription")}
        actions={capabilities.canCreateIncident ? <CreateIncidentDialog teamId={teamId} /> : null}
      />

      {!hasNoTeams ? (
        <PageTabs
          ariaLabel={t("viewsLabel")}
          tabs={VIEWS.map((tab) => ({
            href: tabHref(tab),
            label: t(`view${tab[0].toUpperCase()}${tab.slice(1)}`),
            count: counts[tab],
            active: view === tab,
          }))}
        />
      ) : null}

      {!hasNoTeams && (hasIncidents || isLoadingIncidents) ? (
        <PageToolbar aria-label={t("filtersLabel")}>
          <IncidentSearch
            key={urlQuery}
            initialValue={urlQuery}
            label={t("searchLabel")}
            placeholder={t("searchPlaceholder")}
            onCommit={commitSearch}
          />

          <div className="grid grid-cols-1 gap-2 sm:grid-cols-3 lg:flex lg:shrink-0">
            <label>
              <span className="sr-only">{t("severityFilter")}</span>
              <select
                value={severity ?? ""}
                onChange={(event) => setParam("severity", event.target.value || undefined)}
                className="border-border bg-panel text-text focus:border-gold h-9 w-full rounded-md border px-3 text-sm outline-none lg:w-36"
              >
                <option value="">{t("allSeverities")}</option>
                {SEVERITIES.map((value) => (
                  <option key={value} value={value}>
                    {t(`severity${value[0].toUpperCase()}${value.slice(1)}`)}
                  </option>
                ))}
              </select>
            </label>

            <label>
              <span className="sr-only">{t("assigneeFilter")}</span>
              <select
                value={assignee ?? ""}
                onChange={(event) => setParam("assignee", event.target.value || undefined)}
                className="border-border bg-panel text-text focus:border-gold h-9 w-full rounded-md border px-3 text-sm outline-none lg:w-52"
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

            <label>
              <span className="sr-only">{t("sortLabel")}</span>
              <select
                value={sort}
                onChange={(event) => setParam("sort", event.target.value)}
                className="border-border bg-panel text-text focus:border-gold h-9 w-full rounded-md border px-3 text-sm outline-none lg:w-40"
              >
                <option value="newest">{t("sortNewest")}</option>
                <option value="oldest">{t("sortOldest")}</option>
                <option value="severity">{t("sortSeverity")}</option>
              </select>
            </label>
          </div>
        </PageToolbar>
      ) : null}

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
        <IncidentTable incidents={incidents} />
      </PageContent>
    </PageLayout>
  );
}
