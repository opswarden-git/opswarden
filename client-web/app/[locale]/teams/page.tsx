"use client";

import React, { useState } from "react";
import { useTeams } from "@/lib/queries/teams";
import { CreateTeamDialog } from "@/components/teams/CreateTeamDialog";
import { JoinTeamDialog } from "@/components/teams/JoinTeamDialog";
import { RoleChip } from "@/components/teams/RoleChip";
import { TeamRoster } from "@/components/teams/TeamRoster";
import { Shield } from "lucide-react";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/utils";

export default function TeamsPage() {
  const { data: teams, isLoading, error } = useTeams();
  const t = useTranslations("Teams");
  const [activeTeamId, setActiveTeamId] = useState<string>("");

  const selectedTeam = teams?.find((team) => team.team_id === activeTeamId);
  const activeTeam = selectedTeam ?? teams?.[0];

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>
        <div className="flex items-center gap-3">
          <JoinTeamDialog />
          <CreateTeamDialog />
        </div>
      </div>

      {isLoading ? (
        <div className="text-muted animate-pulse py-10 text-center text-sm">{t("loading")}</div>
      ) : error ? (
        <div className="ow-danger rounded-md p-4 text-sm">{t("failedToLoad")}</div>
      ) : teams && teams.length === 0 ? (
        <div className="surface rounded-md p-12 text-center">
          <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
          <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
        </div>
      ) : (
        <div className="grid gap-6 md:grid-cols-3">
          {/* Team switcher — picks which team the roster manages. */}
          <div className="space-y-2 md:col-span-1">
            {teams?.map((team) => {
              const isActive = team.team_id === activeTeam?.team_id;
              return (
                <button
                  key={team.team_id}
                  onClick={() => setActiveTeamId(team.team_id)}
                  className={cn(
                    "flex w-full items-center justify-between gap-2 rounded-md border p-3 text-left transition-colors",
                    isActive
                      ? "border-gold/40 bg-white/[0.05]"
                      : "border-border surface hover:bg-white/[0.03]",
                  )}
                >
                  <span className="text-text truncate font-medium">{team.name}</span>
                  <RoleChip role={team.role} />
                </button>
              );
            })}
          </div>

          {/* Roster = the management surface for the active team. */}
          <div className="md:col-span-2">
            {activeTeam ? (
              <TeamRoster team={activeTeam} onLeftOrDeleted={() => setActiveTeamId("")} />
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}
