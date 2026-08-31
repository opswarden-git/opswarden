"use client";

import React, { useMemo, useState } from "react";
import { ChevronRight, Search, Shield, Users } from "lucide-react";
import { useTranslations } from "next-intl";
import { CreateTeamDialog } from "@/components/teams/CreateTeamDialog";
import { JoinTeamDialog } from "@/components/teams/JoinTeamDialog";
import { RoleChip } from "@/components/teams/RoleChip";
import { PageContent, type PageContentState } from "@/components/layout/PageContent";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageLayout } from "@/components/layout/PageLayout";
import { Alert } from "@/components/ui/Alert";
import { Skeleton } from "@/components/ui/Skeleton";
import { Link } from "@/i18n/routing";
import { useTeamScope } from "@/components/teams/TeamScope";
import { teamPath } from "@/lib/team-routing";

export default function TeamsPage() {
  const { teams, isLoading, error } = useTeamScope();
  const t = useTranslations("Teams");
  const [query, setQuery] = useState("");
  const visibleTeams = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    if (!normalized) return teams ?? [];
    return (teams ?? []).filter((team) => team.name.toLocaleLowerCase().includes(normalized));
  }, [query, teams]);
  const contentState: PageContentState = isLoading
    ? "loading"
    : error
      ? "error"
      : teams?.length === 0
        ? "empty"
        : "ready";

  return (
    <PageLayout>
      <PageHeader
        actions={
          <>
            <label className="relative max-w-72 min-w-52 flex-1">
              <span className="sr-only">{t("searchTeams")}</span>
              <Search
                className="text-muted pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2"
                aria-hidden="true"
              />
              <input
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("searchTeams")}
                className="ow-input h-9 w-full rounded-md pr-3 pl-9 text-sm"
              />
            </label>
            <JoinTeamDialog />
            <CreateTeamDialog />
          </>
        }
      />

      <PageContent
        state={contentState}
        loadingFallback={
          <div
            className="surface divide-border divide-y overflow-hidden rounded-md"
            aria-label={t("loading")}
            aria-busy="true"
          >
            {Array.from({ length: 4 }, (_, index) => (
              <div
                key={index}
                className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-4 py-4 sm:gap-4"
              >
                <Skeleton className="h-9 w-9 rounded-md" />
                <div className="space-y-2">
                  <Skeleton className="h-4 w-40" />
                  <Skeleton className="h-3 w-64 max-w-full" />
                  <Skeleton className="h-5 w-20 rounded-full sm:hidden" />
                </div>
                <div className="flex items-center gap-3">
                  <Skeleton className="hidden h-5 w-20 rounded-full sm:block" />
                  <Skeleton className="h-4 w-4" />
                </div>
              </div>
            ))}
          </div>
        }
        errorFallback={<Alert tone="danger">{t("failedToLoad")}</Alert>}
        emptyFallback={
          <div className="surface rounded-md p-12 text-center">
            <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
            <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
            <p className="text-muted mt-2 text-sm">{t("noTeamsDesc")}</p>
          </div>
        }
      >
        {visibleTeams.length === 0 ? (
          <div className="surface text-muted rounded-md p-12 text-center text-sm">
            {t("noMatchingTeams")}
          </div>
        ) : (
          <div className="surface divide-border divide-y overflow-hidden rounded-md">
            {visibleTeams.map((team) => (
              <Link
                key={team.team_id}
                href={teamPath(team.team_id, "overview")}
                className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-4 py-4 transition-colors hover:bg-white/[0.04] sm:gap-4"
              >
                <span className="surface-subtle border-border flex h-9 w-9 shrink-0 items-center justify-center rounded-md border">
                  <Users className="text-muted h-4 w-4" aria-hidden="true" />
                </span>
                <div className="min-w-0">
                  <p className="text-text truncate font-medium">{team.name}</p>
                  <p className="text-muted mt-0.5 text-xs">
                    {t("directorySummary", {
                      members: team.member_count,
                      incidents: team.active_incident_count,
                      releases: team.active_release_count,
                    })}
                  </p>
                  <div className="mt-2 sm:hidden">
                    <RoleChip role={team.role} />
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-3">
                  <span className="hidden sm:inline-flex">
                    <RoleChip role={team.role} />
                  </span>
                  <ChevronRight className="text-muted h-4 w-4" aria-hidden="true" />
                </div>
              </Link>
            ))}
          </div>
        )}
      </PageContent>
    </PageLayout>
  );
}
