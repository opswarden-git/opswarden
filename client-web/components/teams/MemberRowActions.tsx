"use client";

import React from "react";
import { ChevronDown, ChevronUp, Crown, UserMinus, Ban } from "lucide-react";
import { useTranslations } from "next-intl";
import { TeamMember } from "@/lib/queries/teams";
import { ActionMenu, type ActionMenuEntry } from "@/components/ui/ActionMenu";

/**
 * Labelled management actions for one roster row, shown only to a Manager.
 * The menu primitive owns its portal, keyboard behavior and focus restoration.
 */
export function MemberRowActions({
  member,
  pending,
  onSetRole,
  onMakeManager,
  onKick,
  onBan,
}: {
  member: TeamMember;
  pending: boolean;
  onSetRole: (role: "observer" | "responder") => void;
  onMakeManager: () => void;
  onKick: () => void;
  onBan: () => void;
}) {
  const t = useTranslations("Teams");

  if (member.role === "manager") return null;

  const promote = member.role === "observer";
  const roleLabel = promote ? t("makeResponder") : t("makeObserver");
  const RoleIcon = promote ? ChevronUp : ChevronDown;
  const items: ActionMenuEntry[] = [
    {
      id: "role",
      label: roleLabel,
      icon: RoleIcon,
      onSelect: () => onSetRole(promote ? "responder" : "observer"),
    },
    {
      id: "manager",
      label: t("makeManager"),
      icon: Crown,
      onSelect: onMakeManager,
    },
    { id: "danger-separator", separator: true },
    { id: "kick", label: t("kick"), icon: UserMinus, tone: "danger", onSelect: onKick },
    { id: "ban", label: t("ban"), icon: Ban, tone: "danger", onSelect: onBan },
  ];

  return <ActionMenu label={t("actionsTitle")} items={items} disabled={pending} />;
}
