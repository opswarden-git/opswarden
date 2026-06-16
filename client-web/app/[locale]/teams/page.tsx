"use client";

import React, { useState } from "react";
import { useTeams } from "@/lib/queries/teams";
import { CreateTeamDialog } from "@/components/teams/CreateTeamDialog";
import { JoinTeamDialog } from "@/components/teams/JoinTeamDialog";
import { Shield, ShieldAlert, ShieldCheck, Copy, Check } from "lucide-react";
import { useTranslations } from "next-intl";

export default function TeamsPage() {
  const { data: teams, isLoading, error } = useTeams();
  const [copiedCode, setCopiedCode] = useState<string | null>(null);
  const t = useTranslations("Teams");

  const handleCopy = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(code);
    setTimeout(() => setCopiedCode(null), 2000);
  };

  const getRoleIcon = (role: string) => {
    switch (role) {
      case "manager":
        return <ShieldAlert className="text-gold h-4 w-4" />;
      case "responder":
        return <ShieldCheck className="h-4 w-4 text-blue-400" />;
      default:
        return <Shield className="text-muted h-4 w-4" />;
    }
  };

  return (
    <div className="mx-auto max-w-5xl space-y-8 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-text text-2xl font-bold tracking-tight">{t("title")}</h1>

        </div>
        <div className="flex items-center gap-3">
          <JoinTeamDialog />
          <CreateTeamDialog />
        </div>
      </div>

      {isLoading ? (
        <div className="text-muted animate-pulse py-10 text-center text-sm">{t("loading")}</div>
      ) : error ? (
        <div className="rounded-md border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-500">
          {t("failedToLoad")}
        </div>
      ) : teams && teams.length === 0 ? (
        <div className="rounded-xl border border-white/5 bg-white/5 p-12 text-center">
          <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
          <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl border border-white/5 bg-black/40">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-white/5 bg-white/5 text-xs uppercase">
              <tr>
                <th className="text-muted px-6 py-4 font-medium">{t("colStationName")}</th>
                <th className="text-muted px-6 py-4 font-medium">{t("colRole")}</th>
                <th className="text-muted px-6 py-4 text-right font-medium">{t("colInvitationCode")}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5">
              {teams?.map((team) => (
                <tr key={team.team_id} className="transition-colors hover:bg-white/5">
                  <td className="text-text px-6 py-4 font-medium">{team.name}</td>
                  <td className="px-6 py-4">
                    <div className="text-text inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-xs font-medium capitalize">
                      {getRoleIcon(team.role)}
                      {team.role}
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right">
                    {team.role === "manager" && team.invitation_code ? (
                      <button
                        onClick={() => handleCopy(team.invitation_code!)}
                        className="text-muted hover:text-text inline-flex items-center gap-2 font-mono text-xs transition-colors"
                        title={t("copyInvitationCode")}
                      >
                        {team.invitation_code}
                        {copiedCode === team.invitation_code ? (
                          <Check className="h-4 w-4 text-green-500" />
                        ) : (
                          <Copy className="h-4 w-4" />
                        )}
                      </button>
                    ) : (
                      <span className="text-muted/50 text-xs italic">{t("hidden")}</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
