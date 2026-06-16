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
        return <ShieldCheck className="text-st-ack h-4 w-4" />;
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
        <div className="ow-danger rounded-md p-4 text-sm">{t("failedToLoad")}</div>
      ) : teams && teams.length === 0 ? (
        <div className="surface rounded-md p-12 text-center">
          <Shield className="text-muted/50 mx-auto mb-4 h-12 w-12" />
          <h3 className="text-text text-lg font-medium">{t("noTeamsYet")}</h3>
          <p className="text-muted mt-2 mb-6 text-sm">{t("noTeamsDesc")}</p>
        </div>
      ) : (
        <div className="surface overflow-hidden rounded-md">
          <table className="w-full text-left text-sm">
            <thead className="surface-subtle border-border border-b text-xs uppercase">
              <tr>
                <th className="text-muted px-6 py-4 font-medium">{t("colStationName")}</th>
                <th className="text-muted px-6 py-4 font-medium">{t("colRole")}</th>
                <th className="text-muted px-6 py-4 text-right font-medium">
                  {t("colInvitationCode")}
                </th>
              </tr>
            </thead>
            <tbody className="divide-border divide-y">
              {teams?.map((team) => (
                <tr key={team.team_id} className="transition-colors hover:bg-white/[0.04]">
                  <td className="text-text px-6 py-4 font-medium">{team.name}</td>
                  <td className="px-6 py-4">
                    <div className="surface-subtle text-text border-border inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium capitalize">
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
                          <Check className="text-st-res h-4 w-4" />
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
