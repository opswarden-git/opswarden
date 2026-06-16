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
  const tSidebar = useTranslations("Sidebar");
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
          <h1 className="text-text text-2xl font-bold tracking-tight">{tSidebar("dashboard")}</h1>
        </div>
        {teams && teams.length > 0 && (
          <select
            value={activeTeamId}
            onChange={(e) => setSelectedTeamId(e.target.value)}
            className="ow-input flex h-10 rounded-md px-3 py-2 text-sm transition-colors"
          >
            {teams.map((team) => (
              <option key={team.team_id} value={team.team_id} className="bg-bg text-text">
                {team.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {isLoadingTeams || isLoadingIncidents ? (
        <div className="text-muted animate-pulse text-sm">Loading telemetry...</div>
      ) : !activeTeamId ? (
        <div className="surface rounded-md p-12 text-center">
          <p className="text-muted mb-4">No active station found.</p>
          <Link href="/teams" className="text-gold hover:underline">
            Configure Team
          </Link>
        </div>
      ) : (
        <div className="space-y-8">
          {/* KPI GRID */}
          <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
            <div className="surface flex flex-col justify-between rounded-md p-4">
              <div className="flex items-start justify-between">
                <span className="text-muted text-xs font-medium">Open Incidents</span>
                <AlertTriangle className="text-st-open h-4 w-4" />
              </div>
              <div className="text-text mt-4 font-mono text-3xl font-bold">{openCount}</div>
            </div>

            <div className="surface flex flex-col justify-between rounded-md p-4">
              <div className="flex items-start justify-between">
                <span className="text-muted text-xs font-medium">Acknowledged</span>
                <Activity className="text-st-ack h-4 w-4" />
              </div>
              <div className="text-text mt-4 font-mono text-3xl font-bold">{ackCount}</div>
            </div>

            <div className="surface flex flex-col justify-between rounded-md p-4">
              <div className="flex items-start justify-between">
                <span className="text-muted text-xs font-medium">Escalated</span>
                <Zap className="text-st-esc h-4 w-4" />
              </div>
              <div className="text-text mt-4 font-mono text-3xl font-bold">{escCount}</div>
            </div>

            <div className="surface border-sev-critical/30 flex flex-col justify-between rounded-md p-4 shadow-[inset_0_0_20px_rgba(239,68,68,0.05)]">
              <div className="flex items-start justify-between">
                <span className="text-xs font-medium text-red-400">Critical Sev</span>
                <ShieldAlert className="text-sev-critical h-4 w-4" />
              </div>
              <div className="text-sev-critical mt-4 font-mono text-3xl font-bold">
                {criticalCount}
              </div>
            </div>
          </div>

          {/* RECENT INCIDENTS */}
          <div>
            <div className="mb-4 flex items-end justify-between">
              <h2 className="text-text text-lg font-bold">{tSidebar("incidents")}</h2>
              <Link href="/incidents" className="text-gold text-sm hover:underline">
                View All
              </Link>
            </div>

            {recentIncidents && recentIncidents.length > 0 ? (
              <div className="surface overflow-hidden rounded-md">
                <table className="w-full text-left text-sm">
                  <thead className="surface-subtle border-border border-b text-xs uppercase">
                    <tr>
                      <th className="text-muted px-6 py-4 font-medium">Status</th>
                      <th className="text-muted px-6 py-4 font-medium">Identifier & Title</th>
                      <th className="text-muted px-6 py-4 font-medium">Severity</th>
                      <th className="text-muted px-6 py-4 font-medium">Created At</th>
                      <th className="text-muted px-6 py-4 text-right font-medium">Action</th>
                    </tr>
                  </thead>
                  <tbody className="divide-border divide-y">
                    {recentIncidents.map((incident) => (
                      <IncidentRow key={incident.id} incident={incident} />
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="surface rounded-md p-12 text-center">
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
