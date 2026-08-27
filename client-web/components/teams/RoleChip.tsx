import React from "react";
import { Shield, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslations } from "next-intl";
import type { TeamRole } from "@/lib/capabilities";
import { cn } from "@/lib/utils";

export function RoleIcon({
  className,
  role,
}: {
  className?: string;
  role: TeamRole;
}) {
  const Icon = role === "manager" ? ShieldAlert : role === "responder" ? ShieldCheck : Shield;

  return (
    <Icon className={cn("text-gold", className)} strokeWidth={1.8} aria-hidden="true" />
  );
}

/**
 * Small, reusable RBAC role badge: icon + translated label. Shared by the
 * team list (the user's own role) and the roster (each member's role).
 */
export function RoleChip({
  className,
  role,
  iconOnly = false,
  showIcon = true,
}: {
  className?: string;
  role: TeamRole;
  iconOnly?: boolean;
  showIcon?: boolean;
}) {
  const t = useTranslations("Teams");
  const iconSize = "h-4 w-4";

  const label =
    role === "manager"
      ? t("roleManager")
      : role === "responder"
        ? t("roleResponder")
        : t("roleObserver");

  return (
    <span
      className={cn(
        "text-muted inline-flex shrink-0 items-center gap-1 text-xs leading-4 font-medium",
        className,
      )}
      aria-label={iconOnly ? label : undefined}
    >
      {showIcon ? <RoleIcon role={role} className={iconSize} /> : null}
      {iconOnly ? null : label}
    </span>
  );
}
