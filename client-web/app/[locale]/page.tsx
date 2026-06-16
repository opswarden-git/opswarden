"use client";

import React, { useState } from "react";
import { useTranslations } from "next-intl";
import { useTeams } from "@/lib/queries/teams";
import { useIncidents } from "@/lib/queries/incidents";
import { IncidentRow } from "@/components/incidents/IncidentRow";
import { Activity, AlertTriangle, ShieldAlert, Zap } from "lucide-react";
import { Link } from "@/i18n/routing";

export default function HomePage() {
  const t = useTranslations("Index");
  const { data: teams, isLoading: isLoadingTeams } = useTeams();
  const [selectedTeamId, setSelectedTeamId] = useState<string>("");

  const activeTeamId = selectedTeamId || (teams && teams.length > 0 ? teams[0].team_id : "");
  const { data: incidents, isLoading: isLoadingIncidents } = useIncidents(activeTeamId);

  // Compute KPIs
  const openCount = incidents?.filter((i) => i.status === "open").length || 0;
  const ackCount = incidents?.filter((i) => i.status === "acknowledged").length || 0;
  const escCount = incidents?.filter((i) => i.status === "escalated").length || 0;
  const criticalCount = incidents?.filter((i) => i.severity === "critical").length || 0;

  const recentIncidents = incidents
    ?.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
    .slice(0, 5);

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-text text-2xl font-bold tracking-tight">
            {t("title") || "OpsWarden Cockpit"}
          </h1>
          <p className="text-muted mt-1 text-sm">
            Station Overview
          </p>
        </div>
        {teams && teams.length > 0 && (
          <select
            value={activeTeamId}
            onChange={(e) => setSelectedTeamId(e.target.value)}
            className="focus:border-gold rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-text focus:outline-none transition-colors"
          >
            {teams.map((team) => (
              <option key={team.team_id} value={team.team_id} className="bg-bg">
                {team.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {isLoadingTeams || isLoadingIncidents ? (
        <div className="text-muted animate-pulse text-sm">Loading telemetry...</div>
      ) : !activeTeamId ? (
        <div className="rounded-xl border border-white/5 bg-white/5 p-12 text-center">
          <p className="text-muted mb-4">No active station found.</p>
          <Link href="/teams" className="text-gold hover:underline">Configure Team</Link>
        </div>
      ) : (
        <div className="space-y-8">
          {/* KPI GRID */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="rounded-xl border border-white/5 bg-white/5 p-4 flex flex-col justify-between">
              <div className="flex justify-between items-start">
                <span className="text-muted text-xs font-medium">Open Incidents</span>
                <AlertTriangle className="h-4 w-4 text-st-open" />
              </div>
              <div className="text-3xl font-bold text-text mt-4">{openCount}</div>
            </div>
            
            <div className="rounded-xl border border-white/5 bg-white/5 p-4 flex flex-col justify-between">
              <div className="flex justify-between items-start">
                <span className="text-muted text-xs font-medium">Acknowledged</span>
                <Activity className="h-4 w-4 text-st-ack" />
              </div>
              <div className="text-3xl font-bold text-text mt-4">{ackCount}</div>
            </div>

            <div className="rounded-xl border border-white/5 bg-white/5 p-4 flex flex-col justify-between">
              <div className="flex justify-between items-start">
                <span className="text-muted text-xs font-medium">Escalated</span>
                <Zap className="h-4 w-4 text-st-esc" />
              </div>
              <div className="text-3xl font-bold text-text mt-4">{escCount}</div>
            </div>

            <div className="rounded-xl border border-red-500/20 bg-red-500/10 p-4 flex flex-col justify-between shadow-[inset_0_0_20px_rgba(239,68,68,0.05)]">
              <div className="flex justify-between items-start">
                <span className="text-red-400 text-xs font-medium">Critical Sev</span>
                <ShieldAlert className="h-4 w-4 text-sev-critical" />
              </div>
              <div className="text-3xl font-bold text-red-500 mt-4">{criticalCount}</div>
            </div>
          </div>

          {/* RECENT INCIDENTS */}
          <div>
            <div className="flex justify-between items-end mb-4">
              <h2 className="text-text font-bold text-lg">Active & Recent Streams</h2>
              <Link href="/incidents" className="text-gold text-sm hover:underline">View All</Link>
            </div>
            
            {recentIncidents && recentIncidents.length > 0 ? (
              <div className="overflow-hidden rounded-xl border border-white/5 bg-black/40">
                <table className="w-full text-left text-sm">
                  <thead className="border-b border-white/5 bg-white/5 text-xs uppercase">
                    <tr>
                      <th className="text-muted px-6 py-4 font-medium">Status</th>
                      <th className="text-muted px-6 py-4 font-medium">Identifier & Title</th>
                      <th className="text-muted px-6 py-4 font-medium">Severity</th>
                      <th className="text-muted px-6 py-4 font-medium">Created At</th>
                      <th className="text-muted px-6 py-4 text-right font-medium">Action</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-white/5">
                    {recentIncidents.map((incident) => (
                      <IncidentRow key={incident.id} incident={incident} />
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="rounded-xl border border-white/5 bg-white/5 p-12 text-center">
                <ShieldAlert className="text-muted/50 mx-auto mb-4 h-12 w-12" />
                <p className="text-muted text-sm">No recent incidents detected</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
