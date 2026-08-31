"use client";

import React from "react";
import { useTranslations } from "next-intl";
import type { TeamMember } from "@/lib/queries/teams";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";

export type Dialog = "makeManager" | "kick" | "ban" | null;
export type BanDuration = "permanent" | "1h" | "24h" | "7d";

export function RosterConfirmDialogs({
  banDuration,
  banError,
  banPending,
  dialog,
  kickError,
  kickPending,
  onBanConfirm,
  onClose,
  onKickConfirm,
  onSetBanDuration,
  onTransferConfirm,
  target,
  transferError,
  transferPending,
}: {
  banDuration: BanDuration;
  banError: string | null;
  banPending: boolean;
  dialog: Dialog;
  kickError: string | null;
  kickPending: boolean;
  onBanConfirm: () => void;
  onClose: () => void;
  onKickConfirm: () => void;
  onSetBanDuration: (duration: BanDuration) => void;
  onTransferConfirm: () => void;
  target: TeamMember | null;
  transferError: string | null;
  transferPending: boolean;
}) {
  const t = useTranslations("Teams");
  return (
    <>
      <ConfirmDialog
        open={dialog === "makeManager"}
        title={t("makeManager")}
        description={t("transferConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("makeManager")}
        cancelLabel={t("cancel")}
        intent="standard"
        pendingLabel={t("processing")}
        pending={transferPending}
        error={transferError}
        onConfirm={onTransferConfirm}
        onClose={onClose}
      />
      <ConfirmDialog
        open={dialog === "kick"}
        title={t("kick")}
        description={t("kickConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("kick")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("processing")}
        pending={kickPending}
        error={kickError}
        onConfirm={onKickConfirm}
        onClose={onClose}
      />
      <ConfirmDialog
        open={dialog === "ban"}
        title={t("banMember")}
        description={t("banConfirm", { email: target?.email ?? "" })}
        confirmLabel={t("ban")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pendingLabel={t("processing")}
        pending={banPending}
        error={banError}
        onConfirm={onBanConfirm}
        onClose={onClose}
      >
        <label className="space-y-2">
          <span className="text-muted text-sm">{t("banDuration")}</span>
          <select
            value={banDuration}
            onChange={(event) => onSetBanDuration(event.target.value as BanDuration)}
            className="ow-input h-10 w-full rounded-md px-3 text-sm"
          >
            <option value="permanent">{t("banPermanent")}</option>
            <option value="1h">{t("ban1h")}</option>
            <option value="24h">{t("ban24h")}</option>
            <option value="7d">{t("ban7d")}</option>
          </select>
        </label>
      </ConfirmDialog>
    </>
  );
}
