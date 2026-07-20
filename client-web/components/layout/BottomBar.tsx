"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import { cn } from "@/lib/utils";
import { useTranslations } from "next-intl";
import {
  isNavigationItemActive,
  primaryNavigationItems,
  settingsNavigationItem,
} from "./navigation";
import { useTeamScope } from "@/components/teams/TeamScope";

export function BottomBar({ className }: { className?: string }) {
  const pathname = usePathname();
  const t = useTranslations("Sidebar");
  const { activeTeam } = useTeamScope();
  const links = [...primaryNavigationItems(activeTeam?.team_id), settingsNavigationItem];

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

        return (
          <Link
            key={link.labelKey}
            href={link.href}
            aria-current={isActive ? "page" : undefined}
            data-app-navigation-item="true"
            className={cn(
              "flex h-full min-w-0 flex-1 flex-col items-center justify-center gap-1 transition-colors",
              isActive ? "text-gold" : "text-muted hover:text-gold",
            )}
          >
            <link.icon className="h-5 w-5" aria-hidden="true" />
            <span className="w-full truncate px-0.5 text-center text-[10px] font-medium">
              {t(link.labelKey)}
            </span>
          </Link>
        );
      })}
    </nav>
  );
}
