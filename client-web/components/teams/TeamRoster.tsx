"use client";

import React, { useState } from "react";
import { useTranslations } from "next-intl";
import { Users, Copy, Check, Trash2, LogOut } from "lucide-react";
import {
  Team,
  TeamMember,
  useTeamMembers,
  useSetMemberRole,
  useTransferManager,
  useLeaveTeam,
  useDeleteTeam,
} from "@/lib/queries/teams";
import { useTeamOnline } from "@/lib/ws";
import { RoleChip } from "./RoleChip";
import { MemberRowActions } from "./MemberRowActions";
import { ConfirmTeamActionDialog } from "./ConfirmTeamActionDialog";

/** Avatar initials derived from the email local-part (e.g. romeo.cavazza → RC). */
function initials(email: string): string {
  const local = email.split("@")[0] ?? email;
  const parts = local.split(/[._-]+/).filter(Boolean);
  const letters = parts.length >= 2 ? parts[0][0] + parts[1][0] : local.slice(0, 2);
  return letters.toUpperCase();
}

type Dialog = "leave" | "delete" | "makeManager" | null;

/**
 * The roster IS the team-management surface: members list + (Manager-only)
 * per-row role actions, plus a slim danger zone footer. Replaces the old
 * floating TeamActions block.
 */
export function TeamRoster({ team, onLeftOrDeleted }: { team: Team; onLeftOrDeleted: () => void }) {
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");

  const { data: members, isLoading, error } = useTeamMembers(team.team_id);
  const onlineSet = new Set(useTeamOnline(team.team_id));
  // Count only members actually shown in the roster (avoids counting a stale
  // online id that has already left, before the roster query refetches).
  const onlineCount = (members ?? []).filter((m) => onlineSet.has(m.user_id)).length;
  const setRole = useSetMemberRole(team.team_id);
  const transfer = useTransferManager(team.team_id);
  const leave = useLeaveTeam(team.team_id);
  const remove = useDeleteTeam(team.team_id);

  const [dialog, setDialog] = useState<Dialog>(null);
  const [target, setTarget] = useState<TeamMember | null>(null);
  const [copied, setCopied] = useState(false);

  const isManager = team.role === "manager";
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const close = () => setDialog(null);

  const copyInvite = () => {
    if (!team.invitation_code) return;
    navigator.clipboard.writeText(team.invitation_code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const openMakeManager = (member: TeamMember) => {
    transfer.reset();
    setTarget(member);
    setDialog("makeManager");
  };
  const onMakeManager = () => {
    if (target) transfer.mutate(target.user_id, { onSuccess: close });
  };
  const onLeave = () =>
    leave.mutate(undefined, {
      onSuccess: () => {
        close();
        onLeftOrDeleted();
      },
    });
  const onDelete = () =>
    remove.mutate(undefined, {
      onSuccess: () => {
        close();
        onLeftOrDeleted();
      },
    });

  return (
    <div className="surface overflow-hidden rounded-md">
      <div className="border-border flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-6 py-4">
        <Users className="text-muted h-5 w-5" />
        <h2 className="text-text text-lg font-semibold tracking-tight">{t("members")}</h2>
        <span className="text-muted/70 truncate text-sm">— {team.name}</span>
        {onlineCount > 0 ? (
          <span className="text-muted/60 inline-flex items-center gap-1.5 text-xs">
            <span className="bg-st-res h-1.5 w-1.5 rounded-full" />
            {t("onlineCount", { count: onlineCount })}
          </span>
        ) : null}
        {isManager && team.invitation_code ? (
          <button
            type="button"
            onClick={copyInvite}
            title={t("copyInvitationCode")}
            className="text-muted hover:text-text ml-auto inline-flex items-center gap-2 font-mono text-xs transition-colors"
          >
            {team.invitation_code}
            {copied ? (
              <Check className="text-st-res h-3.5 w-3.5" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </button>
        ) : null}
      </div>

      {setRole.error ? (
        <div className="ow-danger m-4 rounded-md p-3 text-sm">
          {errorText(setRole.error.message)}
        </div>
      ) : null}

      {isLoading ? (
        <div className="divide-border divide-y">
          {[0, 1, 2].map((i) => (
            <div key={i} className="flex items-center gap-3 px-6 py-4">
              <div className="bg-muted/20 h-8 w-8 animate-pulse rounded-full" />
              <div className="bg-muted/20 h-4 w-48 animate-pulse rounded" />
              <div className="bg-muted/20 ml-auto h-5 w-20 animate-pulse rounded-full" />
            </div>
          ))}
        </div>
      ) : error ? (
        <div className="ow-danger m-4 rounded-md p-4 text-sm">{t("membersFailed")}</div>
      ) : members && members.length === 0 ? (
        <div className="text-muted px-6 py-10 text-center text-sm">{t("noMembers")}</div>
      ) : (
        <ul className="divide-border divide-y">
          {members?.map((member) => (
            <li
              key={member.user_id}
              className="flex items-center gap-3 px-6 py-4 transition-colors hover:bg-white/[0.03]"
            >
              <span className="relative shrink-0">
                <span className="surface-subtle text-muted border-border flex h-8 w-8 items-center justify-center rounded-full border text-xs font-semibold">
                  {initials(member.email)}
                </span>
                <span
                  title={onlineSet.has(member.user_id) ? t("online") : t("offline")}
                  aria-label={onlineSet.has(member.user_id) ? t("online") : t("offline")}
                  className={`border-bg absolute -right-0.5 -bottom-0.5 h-2.5 w-2.5 rounded-full border-2 ${
                    onlineSet.has(member.user_id) ? "bg-st-res" : "bg-muted/40"
                  }`}
                />
              </span>
              <div className="min-w-0 flex-1">
                <div className="text-text truncate font-medium">{member.email}</div>
                <div className="text-muted/50 font-mono text-xs">
                  {member.user_id.split("-")[0]}
                </div>
              </div>
              <RoleChip role={member.role} />
              {isManager ? (
                <MemberRowActions
                  member={member}
                  pending={setRole.isPending || transfer.isPending}
                  onSetRole={(role) => setRole.mutate({ userId: member.user_id, role })}
                  onMakeManager={() => openMakeManager(member)}
                />
              ) : null}
            </li>
          ))}
        </ul>
      )}

      <div className="border-border flex items-center justify-between gap-4 border-t bg-white/[0.015] px-6 py-4">
        <div className="min-w-0">
          <h3 className="text-muted/70 text-xs font-medium tracking-wider uppercase">
            {t("dangerZone")}
          </h3>
          <p className="text-muted/60 mt-1 truncate text-xs">
            {isManager ? t("deleteTeamDesc") : t("leaveTeamDesc")}
          </p>
        </div>
        {isManager ? (
          <button
            type="button"
            onClick={() => {
              remove.reset();
              setDialog("delete");
            }}
            className="ow-danger inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md px-3 text-sm font-medium whitespace-nowrap transition-colors"
          >
            <Trash2 className="h-4 w-4" />
            {t("deleteTeam")}
          </button>
        ) : (
          <button
            type="button"
            onClick={() => {
              leave.reset();
              setDialog("leave");
            }}
            className="ow-danger inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md px-3 text-sm font-medium whitespace-nowrap transition-colors"
          >
            <LogOut className="h-4 w-4" />
            {t("leaveTeam")}
          </button>
        )}
      </div>

      <ConfirmTeamActionDialog
        open={dialog === "makeManager"}
        title={t("makeManager")}
        description={t("transferConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("makeManager")}
        pending={transfer.isPending}
        error={transfer.error ? errorText(transfer.error.message) : null}
        onConfirm={onMakeManager}
        onClose={close}
      />
      <ConfirmTeamActionDialog
        open={dialog === "leave"}
        title={t("leaveTeam")}
        description={t("leaveConfirm", { name: team.name })}
        confirmLabel={t("leaveTeam")}
        danger
        pending={leave.isPending}
        error={leave.error ? errorText(leave.error.message) : null}
        onConfirm={onLeave}
        onClose={close}
      />
      <ConfirmTeamActionDialog
        open={dialog === "delete"}
        title={t("deleteTeam")}
        description={t("deleteConfirm", { name: team.name })}
        confirmLabel={t("deleteTeam")}
        danger
        requireType="DELETE"
        pending={remove.isPending}
        error={remove.error ? errorText(remove.error.message) : null}
        onConfirm={onDelete}
        onClose={close}
      />
    </div>
  );
}
