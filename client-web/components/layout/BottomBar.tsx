"use client";

import React from "react";
import { CircleUser } from "lucide-react";
import { Link, usePathname } from "@/i18n/routing";
import { cn } from "@/lib/utils";
import { useTranslations } from "next-intl";
import {
  isNavigationItemActive,
  primaryNavigationItems,
  settingsNavigationItem,
} from "./navigation";
import { useTeamScope } from "@/components/teams/TeamScope";
import { deriveCapabilities } from "@/lib/capabilities";

export function BottomBar({ className }: { className?: string }) {
  const pathname = usePathname();
  const t = useTranslations("Sidebar");
  const { activeTeam } = useTeamScope();
  const canViewActivity = activeTeam
    ? deriveCapabilities(activeTeam.role).canManageAutomations
    : false;
  const links = [
    ...primaryNavigationItems(activeTeam?.team_id, canViewActivity),
    settingsNavigationItem,
  ];

  return (
    <nav
      aria-label={t("mobileNavigation")}
      className={cn(
        "glass fixed right-0 bottom-0 left-0 z-50 flex h-16 items-center justify-around px-2",
        className,
      )}
    >
      {links.map((link) => {
        const isActive = isNavigationItemActive(pathname, link);
        // The account is not a section of the product. The sidebar already says
        // so by putting it in the footer under the operator's name instead of
        // among the navigation entries; here it earns the identity icon and a
        // rule to its left, so it stops reading as a peer of Incidents.
        const isAccount = link.labelKey === settingsNavigationItem.labelKey;
        const Icon = isAccount ? CircleUser : link.icon;

        return (
          <Link
            key={link.labelKey}
            href={link.href}
            aria-current={isActive ? "page" : undefined}
            data-app-navigation-item="true"
            className={cn(
              "flex h-full min-w-0 flex-1 flex-col items-center justify-center gap-1 transition-colors",
              isAccount && "border-border border-l",
              isActive ? "text-gold" : "text-muted hover:text-gold",
            )}
          >
            <Icon className="h-5 w-5" aria-hidden="true" />
            <span className="w-full truncate px-0.5 text-center text-[10px] font-medium">
              {t(link.labelKey)}
            </span>
          </Link>
        );
      })}
    </nav>
  );
}
