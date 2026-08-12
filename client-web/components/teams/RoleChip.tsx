import React from "react";
import { Shield, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslations } from "next-intl";
import type { TeamRole } from "@/lib/capabilities";

/**
 * Small, reusable RBAC role badge: icon + translated label. Shared by the
 * team list (the user's own role) and the roster (each member's role).
 */
export function RoleChip({ role, iconOnly = false }: { role: TeamRole; iconOnly?: boolean }) {
  const t = useTranslations("Teams");
  const iconSize = iconOnly ? "h-4 w-4" : "h-3 w-3";

  const icon =
    role === "manager" ? (
      <ShieldAlert className={`text-gold ${iconSize}`} aria-hidden="true" />
    ) : role === "responder" ? (
      <ShieldCheck className={`text-st-ack ${iconSize}`} aria-hidden="true" />
    ) : (
      <Shield className={`text-muted ${iconSize}`} aria-hidden="true" />
    );

  const label =
    role === "manager"
      ? t("roleManager")
      : role === "responder"
        ? t("roleResponder")
        : t("roleObserver");

  return (
    <span
      className="text-muted inline-flex shrink-0 items-center gap-1 text-xs leading-4 font-medium"
      aria-label={iconOnly ? label : undefined}
    >
      {icon}
      {iconOnly ? null : label}
    </span>
  );
}
