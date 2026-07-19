import React from "react";
import { Shield, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslations } from "next-intl";
import type { TeamRole } from "@/lib/capabilities";

/**
 * Small, reusable RBAC role badge: icon + translated label. Shared by the
 * team list (the user's own role) and the roster (each member's role).
 */
export function RoleChip({ role }: { role: TeamRole }) {
  const t = useTranslations("Teams");

  const icon =
    role === "manager" ? (
      <ShieldAlert className="text-gold h-3.5 w-3.5" />
    ) : role === "responder" ? (
      <ShieldCheck className="text-st-ack h-3.5 w-3.5" />
    ) : (
      <Shield className="text-muted h-3.5 w-3.5" />
    );

  const label =
    role === "manager"
      ? t("roleManager")
      : role === "responder"
        ? t("roleResponder")
        : t("roleObserver");

  return (
    <span className="surface-subtle text-text border-border inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium">
      {icon}
      {label}
    </span>
  );
}
