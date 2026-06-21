"use client";

import React, { useState } from "react";
import { useTeams, useTeamMembers } from "@/lib/queries/teams";
import { CreateTeamDialog } from "@/components/teams/CreateTeamDialog";
import { JoinTeamDialog } from "@/components/teams/JoinTeamDialog";
import { RoleChip } from "@/components/teams/RoleChip";
import { TeamActions } from "@/components/teams/TeamActions";
import { Shield, Users, Copy, Check } from "lucide-react";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/utils";

/** Avatar initials derived from the email local-part (e.g. romeo.cavazza → RC). */
function initials(email: string): string {
  const local = email.split("@")[0] ?? email;
  const parts = local.split(/[._-]+/).filter(Boolean);
  const letters = parts.length >= 2 ? parts[0][0] + parts[1][0] : local.slice(0, 2);
  return letters.toUpperCase();
}

export default function TeamsPage() {
  const { data: teams, isLoading, error } = useTeams();
  const t = useTranslations("Teams");
  const [activeTeamId, setActiveTeamId] = useState<string>("");
  const [copiedCode, setCopiedCode] = useState<string | null>(null);

  const selectedTeam = teams?.find((team) => team.team_id === activeTeamId);
  const activeTeam = selectedTeam ?? teams?.[0];
  const effectiveTeamId = activeTeam?.team_id || "";
  const {
    data: members,
    isLoading: membersLoading,
    error: membersError,
  } = useTeamMembers(effectiveTeamId || undefined);

  const handleCopy = (code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(code);
    setTimeout(() => setCopiedCode(null), 2000);
  };

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
          {/* Team list — selecting one drives the roster on the right. */}
          <div className="space-y-2 md:col-span-1">
            {teams?.map((team) => {
              const isActive = team.team_id === effectiveTeamId;
              return (
                <div
                  key={team.team_id}
                  className={cn(
                    "rounded-md border p-2 transition-colors",
                    isActive
                      ? "border-gold/40 bg-white/[0.05]"
                      : "border-border surface hover:bg-white/[0.03]",
                  )}
                >
                  <button
                    onClick={() => setActiveTeamId(team.team_id)}
                    className="flex w-full items-center justify-between gap-2 rounded px-2 py-2 text-left"
                  >
                    <span className="text-text truncate font-medium">{team.name}</span>
                    <RoleChip role={team.role} />
                  </button>
                  {team.role === "manager" && team.invitation_code ? (
                    <button
                      onClick={() => handleCopy(team.invitation_code!)}
                      title={t("copyInvitationCode")}
                      className="text-muted hover:text-text inline-flex items-center gap-2 px-2 pb-1 font-mono text-xs transition-colors"
                    >
                      {team.invitation_code}
                      {copiedCode === team.invitation_code ? (
                        <Check className="text-st-res h-3.5 w-3.5" />
                      ) : (
                        <Copy className="h-3.5 w-3.5" />
                      )}
                    </button>
                  ) : null}
                </div>
              );
            })}
          </div>

          {/* Roster for the active team. */}
          <div className="surface overflow-hidden rounded-md md:col-span-2">
            <div className="border-border flex items-center gap-2 border-b px-6 py-4">
              <Users className="text-muted h-5 w-5" />
              <h2 className="text-text text-lg font-semibold tracking-tight">{t("members")}</h2>
              {activeTeam ? (
                <span className="text-muted/70 truncate text-sm">— {activeTeam.name}</span>
              ) : null}
            </div>

            {membersLoading ? (
              <div className="divide-border divide-y">
                {[0, 1, 2].map((i) => (
                  <div key={i} className="flex items-center gap-3 px-6 py-4">
                    <div className="bg-muted/20 h-8 w-8 animate-pulse rounded-full" />
                    <div className="bg-muted/20 h-4 w-48 animate-pulse rounded" />
                    <div className="bg-muted/20 ml-auto h-5 w-20 animate-pulse rounded-full" />
                  </div>
                ))}
              </div>
            ) : membersError ? (
              <div className="ow-danger m-4 rounded-md p-4 text-sm">{t("membersFailed")}</div>
            ) : members && members.length === 0 ? (
              <div className="text-muted px-6 py-10 text-center text-sm">{t("noMembers")}</div>
            ) : (
              <table className="w-full text-left text-sm">
                <thead className="surface-subtle border-border border-b text-xs uppercase">
                  <tr>
                    <th className="text-muted px-6 py-3 font-medium">{t("colMember")}</th>
                    <th className="text-muted px-6 py-3 text-right font-medium">{t("colRole")}</th>
                  </tr>
                </thead>
                <tbody className="divide-border divide-y">
                  {members?.map((member) => (
                    <tr key={member.user_id} className="transition-colors hover:bg-white/[0.03]">
                      <td className="px-6 py-4">
                        <div className="flex items-center gap-3">
                          <span className="surface-subtle text-muted border-border flex h-8 w-8 shrink-0 items-center justify-center rounded-full border text-xs font-semibold">
                            {initials(member.email)}
                          </span>
                          <div className="min-w-0">
                            <div className="text-text truncate font-medium">{member.email}</div>
                            <div className="text-muted/50 font-mono text-xs">
                              {member.user_id.split("-")[0]}
                            </div>
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-4">
                        <div className="flex justify-end">
                          <RoleChip role={member.role} />
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      )}

      {activeTeam ? (
        <TeamActions
          team={activeTeam}
          members={members ?? []}
          onLeftOrDeleted={() => setActiveTeamId("")}
        />
      ) : null}
    </div>
  );
}
