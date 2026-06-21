"use client";

import React from "react";
import { ChevronDown, ChevronUp, Crown } from "lucide-react";
import { useTranslations } from "next-intl";
import { TeamMember } from "@/lib/queries/teams";

/**
 * Compact inline management actions for one roster row, shown only to a Manager.
 * Deliberately inline icon-buttons (with title/aria) rather than a popover menu:
 * the roster card clips overflow, so a faux-dropdown would be fragile — two
 * sober icons read better than a janky menu. The Manager's own row has no
 * actions (the Manager seat only moves through transfer).
 */
export function MemberRowActions({
  member,
  pending,
  onSetRole,
  onMakeManager,
}: {
  member: TeamMember;
  pending: boolean;
  onSetRole: (role: "observer" | "responder") => void;
  onMakeManager: () => void;
}) {
  const t = useTranslations("Teams");

  if (member.role === "manager") return null;

  const promote = member.role === "observer";
  const roleLabel = promote ? t("makeResponder") : t("makeObserver");

  return (
    <div className="flex items-center justify-end gap-1">
      <button
        type="button"
        onClick={() => onSetRole(promote ? "responder" : "observer")}
        disabled={pending}
        title={roleLabel}
        aria-label={roleLabel}
        className="text-muted hover:text-text rounded-md p-1.5 transition-colors hover:bg-white/[0.06] disabled:opacity-40"
      >
        {promote ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
      </button>
      <button
        type="button"
        onClick={onMakeManager}
        disabled={pending}
        title={t("makeManager")}
        aria-label={t("makeManager")}
        className="text-muted hover:text-gold rounded-md p-1.5 transition-colors hover:bg-white/[0.06] disabled:opacity-40"
      >
        <Crown className="h-4 w-4" />
      </button>
    </div>
  );
}
