"use client";

import React, { useState, useEffect } from "react";
import { useTeams } from "@/lib/queries/teams";
import { useIncidents } from "@/lib/queries/incidents";
import { CreateIncidentDialog } from "@/components/incidents/CreateIncidentDialog";
import { IncidentRow } from "@/components/incidents/IncidentRow";
import { AlertCircle, Shield } from "lucide-react";
import { Link } from "@/i18n/routing";
import { useTranslations } from "next-intl";

export default function IncidentsPage() {
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
  const [selectedTeamId, setSelectedTeamId] = useState<string>("");
  const t = useTranslations("Incidents");

  const activeTeamId = selectedTeamId || (teams && teams.length > 0 ? teams[0].team_id : "");

  const { data: incidents, isLoading: isLoadingIncidents, error } = useIncidents(activeTeamId);

  if (isLoadingTeams)
    return <div className="text-muted animate-pulse p-10 text-center">{t("loading")}</div>;

  if (teams && teams.length === 0) {
    return (
      <div className="mx-auto max-w-5xl space-y-8 p-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>

          </div>
        </div>
        <div className="rounded-xl border border-white/5 bg-white/5 p-12 text-center">
          <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noStation")}</h3>
          <p className="text-muted mt-2 mb-6 text-sm">{t("noStationDesc")}</p>
          <Link
            href="/teams"
            className="bg-gold text-bg hover:bg-gold-hover inline-flex items-center rounded-md px-4 py-2 font-sans text-sm font-bold transition-colors"
          >
            {t("goToTeams")}
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>

        </div>
        <div className="flex items-center gap-4">
          <select
            value={activeTeamId}
            onChange={(e) => setSelectedTeamId(e.target.value)}
            className="focus:border-gold rounded-md border border-white/10 bg-black/50 px-3 py-2 text-sm text-white focus:outline-none"
          >
            <option value="" disabled>
              {t("selectStation")}
            </option>
            {teams?.map((team) => (
              <option key={team.team_id} value={team.team_id}>
                {team.name}
              </option>
            ))}
          </select>
          <CreateIncidentDialog teamId={activeTeamId} />
        </div>
      </div>

      {isLoadingIncidents ? (
        <div className="text-muted animate-pulse py-10 text-center text-sm">
          {t("loading")}
        </div>
      ) : error ? (
        <div className="rounded-md border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-500">
          {t("failedToLoad")}
        </div>
      ) : incidents && incidents.length === 0 ? (
        <div className="rounded-xl border border-white/5 bg-white/5 p-12 text-center">
          <AlertCircle className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noIncidentsYet")}</h3>
          <p className="text-muted mt-2 text-sm">{t("noIncidentsDesc")}</p>
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl border border-white/5 bg-black/40">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-white/5 bg-white/5 text-xs uppercase">
              <tr>
                <th className="text-muted px-4 py-3 font-medium">{t("colStatus")}</th>
                <th className="text-muted px-4 py-3 font-medium">{t("colTitleId")}</th>
                <th className="text-muted px-4 py-3 font-medium">{t("colSeverity")}</th>
                <th className="text-muted px-4 py-3 font-medium">{t("colCreatedAt")}</th>
                <th className="text-muted px-4 py-3 text-right font-medium">{t("colAction")}</th>
              </tr>
            </thead>
            <tbody>
              {incidents?.map((incident) => (
                <IncidentRow key={incident.id} incident={incident} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
