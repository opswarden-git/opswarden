"use client";

import React, { useState } from "react";
import { useTeams } from "@/lib/queries/teams";
import { useReleases, useRelease } from "@/lib/queries/releases";
import { CreateReleaseDialog } from "@/components/releases/CreateReleaseDialog";
import { ReleaseStateChip } from "@/components/releases/ReleaseStateChip";
import { ReleaseDetail } from "@/components/releases/ReleaseDetail";
import { Rocket, Shield } from "lucide-react";
import { Link } from "@/i18n/routing";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/utils";

export default function ReleasesPage() {
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
  const [selectedTeamId, setSelectedTeamId] = useState<string>("");
  const [selectedReleaseId, setSelectedReleaseId] = useState<string>("");
  const t = useTranslations("Releases");

  const activeTeamId = selectedTeamId || (teams && teams.length > 0 ? teams[0].team_id : "");
  const activeTeam = teams?.find((team) => team.team_id === activeTeamId);
  const role = activeTeam?.role ?? "observer";

  const { data: releases, isLoading, error } = useReleases(activeTeamId);

  // Fall back to the first release of the active team, and recover gracefully
  // when the previously-selected release belongs to another team.
  const activeReleaseId =
    (releases?.some((r) => r.release_id === selectedReleaseId) ?? false)
      ? selectedReleaseId
      : (releases?.[0]?.release_id ?? "");
  const { data: activeRelease } = useRelease(activeReleaseId || undefined);

  if (isLoadingTeams)
    return <div className="text-muted animate-pulse p-10 text-center">{t("loading")}</div>;

  if (teams && teams.length === 0) {
    return (
      <div className="mx-auto max-w-5xl space-y-8 p-6">
        <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>
        <div className="surface rounded-md p-12 text-center">
          <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
          <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
          <Link
            href="/teams"
            className="ow-primary inline-flex h-10 items-center justify-center rounded-md px-4 text-sm font-medium transition-colors"
          >
            {t("goToTeams")}
          </Link>
        </div>
      </div>
    );
  }

  const canCreate = role === "manager" || role === "responder";

  return (
    <div className="mx-auto max-w-6xl space-y-8 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>
        <div className="flex items-center gap-4">
          <select
            value={activeTeamId}
            onChange={(e) => {
              setSelectedTeamId(e.target.value);
              setSelectedReleaseId("");
            }}
            className="ow-input flex h-10 rounded-md px-3 py-2 text-sm transition-colors"
          >
            {teams?.map((team) => (
              <option key={team.team_id} value={team.team_id} className="bg-bg text-text">
                {team.name}
              </option>
            ))}
          </select>
          {canCreate ? <CreateReleaseDialog teamId={activeTeamId} /> : null}
        </div>
      </div>

      {isLoading ? (
        <div className="text-muted animate-pulse py-10 text-center text-sm">{t("loading")}</div>
      ) : error ? (
        <div className="ow-danger rounded-md p-4 text-sm">{t("failedToLoad")}</div>
      ) : releases && releases.length === 0 ? (
        <div className="surface rounded-md p-12 text-center">
          <Rocket className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noReleasesYet")}</h3>
          <p className="text-muted mt-2 text-sm">{t("noReleasesDesc")}</p>
        </div>
      ) : (
        <div className="grid gap-6 md:grid-cols-3">
          <div className="space-y-2 md:col-span-1">
            {releases?.map((release) => {
              const isActive = release.release_id === activeReleaseId;
              return (
                <button
                  key={release.release_id}
                  onClick={() => setSelectedReleaseId(release.release_id)}
                  className={cn(
                    "flex w-full flex-col gap-2 rounded-md border p-3 text-left transition-colors",
                    isActive
                      ? "border-gold/40 bg-white/[0.05]"
                      : "border-border surface hover:bg-white/[0.03]",
                  )}
                >
                  <span className="text-text truncate font-medium">{release.title}</span>
                  <ReleaseStateChip state={release.state} />
                </button>
              );
            })}
          </div>

          <div className="md:col-span-2">
            {activeRelease ? (
              <ReleaseDetail release={activeRelease} teamId={activeTeamId} role={role} />
            ) : (
              <div className="surface text-muted rounded-md p-10 text-center text-sm">
                {t("selectRelease")}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
