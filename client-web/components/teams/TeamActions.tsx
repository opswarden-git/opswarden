"use client";

import React, { useState } from "react";
import { useTranslations } from "next-intl";
import { LogOut, Trash2, UserCog } from "lucide-react";
import {
  Team,
  TeamMember,
  useDeleteTeam,
  useLeaveTeam,
  useTransferManager,
} from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { ConfirmTeamActionDialog } from "./ConfirmTeamActionDialog";

type Dialog = "leave" | "delete" | "transfer" | null;

/**
 * Compact ownership/membership actions for the active team. Manager sees
 * Transfer + Delete (and a hint that they leave by transferring or deleting);
 * everyone else sees Leave. Backend endpoints already enforce the RBAC — the UI
 * just gates which controls show and confirms destructive ones.
 */
export function TeamActions({
  team,
  members,
  onLeftOrDeleted,
}: {
  team: Team;
  members: TeamMember[];
  onLeftOrDeleted: () => void;
}) {
  const t = useTranslations("Teams");
  const tErr = useTranslations("errors");
  const userId = useAuthStore((s) => s.user?.id);

  const leave = useLeaveTeam(team.team_id);
  const remove = useDeleteTeam(team.team_id);
  const transfer = useTransferManager(team.team_id);

  const [dialog, setDialog] = useState<Dialog>(null);
  const [newManagerId, setNewManagerId] = useState("");

  const isManager = team.role === "manager";
  const others = members.filter((m) => m.user_id !== userId);
  const selectedManager = others.find((m) => m.user_id === newManagerId);

  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));
  const close = () => setDialog(null);
  const openDialog = (next: Exclude<Dialog, null>) => {
    leave.reset();
    remove.reset();
    transfer.reset();
    setDialog(next);
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

  const onTransfer = () => {
    if (!newManagerId) return;
    transfer.mutate(newManagerId, {
      onSuccess: () => {
        close();
        setNewManagerId("");
      },
    });
  };

  return (
    <div className="surface rounded-md p-6">
      <h2 className="text-text border-border flex items-center gap-2 border-b pb-4 text-lg font-semibold tracking-tight">
        <UserCog className="text-muted h-5 w-5" />
        {t("actionsTitle")}
      </h2>

      {isManager ? (
        <div className="mt-4 space-y-6">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
            <label className="flex-1">
              <span className="text-muted/70 mb-1 block text-xs font-medium tracking-wider uppercase">
                {t("transferManager")}
              </span>
              <select
                value={newManagerId}
                onChange={(e) => setNewManagerId(e.target.value)}
                className="ow-input flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
              >
                <option value="" className="bg-bg text-text">
                  {t("transferPickMember")}
                </option>
                {others.map((member) => (
                  <option key={member.user_id} value={member.user_id} className="bg-bg text-text">
                    {member.email}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              onClick={() => openDialog("transfer")}
              disabled={!newManagerId || transfer.isPending}
              className="ow-secondary text-text hover:border-gold/50 inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50"
            >
              <UserCog className="h-4 w-4" />
              {t("transfer")}
            </button>
          </div>
          {others.length === 0 ? (
            <p className="text-muted/60 text-xs">{t("transferNeedsMember")}</p>
          ) : null}

          <div className="border-border flex items-center justify-between gap-4 border-t pt-4">
            <div className="min-w-0">
              <h3 className="text-sm font-medium text-red-400">{t("deleteTeam")}</h3>
              <p className="text-muted/60 mt-1 text-xs">{t("deleteTeamDesc")}</p>
            </div>
            <button
              type="button"
              onClick={() => openDialog("delete")}
              className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors"
            >
              <Trash2 className="h-4 w-4" />
              {t("deleteTeam")}
            </button>
          </div>
        </div>
      ) : (
        <div className="mt-4 flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-sm font-medium text-red-400">{t("leaveTeam")}</h3>
            <p className="text-muted/60 mt-1 text-xs">{t("leaveTeamDesc")}</p>
          </div>
          <button
            type="button"
            onClick={() => openDialog("leave")}
            className="ow-danger inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium whitespace-nowrap transition-colors"
          >
            <LogOut className="h-4 w-4" />
            {t("leaveTeam")}
          </button>
        </div>
      )}

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

      <ConfirmTeamActionDialog
        open={dialog === "transfer"}
        title={t("transferManager")}
        description={t("transferConfirm", { email: selectedManager?.email ?? "" })}
        confirmLabel={t("transfer")}
        pending={transfer.isPending}
        error={transfer.error ? errorText(transfer.error.message) : null}
        onConfirm={onTransfer}
        onClose={close}
      />
    </div>
  );
}
