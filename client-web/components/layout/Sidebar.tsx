"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import { CircleUser, LogOut, Settings } from "lucide-react";
import Image from "next/image";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/store/auth";
import { useTranslations } from "next-intl";
import {
  isNavigationItemActive,
  primaryNavigationItems,
  settingsNavigationItem,
} from "./navigation";
import { IconButton } from "@/components/ui/Button";
import { useTeamScope } from "@/components/teams/TeamScope";

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();
  const t = useTranslations("Sidebar");
  const user = useAuthStore((state) => state.user);
  const { activeTeam, hrefFor } = useTeamScope();
  const navigationItems = primaryNavigationItems(activeTeam?.team_id);
  const isSettingsActive = isNavigationItemActive(pathname, settingsNavigationItem);

  return (
    <aside className={cn("glass flex w-80 flex-col", className)}>
      <Link
        href={activeTeam ? hrefFor("incidents") : "/teams"}
        className="flex h-28 w-full shrink-0 items-center justify-start gap-4 px-8 pt-4 transition-opacity hover:opacity-80"
      >
        <Image
          src="/assets/logo-icon.png"
          alt="Icon"
          width={39}
          height={32}
          className="object-contain"
          priority
        />
        <Image
          src="/assets/logo-text-light.png"
          alt="OpsWarden"
          width={181}
          height={28}
          className="object-contain object-left"
          priority
        />
      </Link>

      <nav
        aria-label={t("primaryNavigation")}
        className="flex-1 space-y-2 overflow-y-auto px-4 py-6"
      >
        {navigationItems.map((link) => {
          const isActive = isNavigationItemActive(pathname, link);

          return (
            <Link
              key={link.labelKey}
              href={link.href}
              aria-current={isActive ? "page" : undefined}
              data-app-navigation-item="true"
              className={cn(
                "group flex items-center gap-4 px-4 py-3 text-lg transition-colors",
                isActive ? "text-gold font-medium" : "text-muted hover:text-gold",
              )}
            >
              <link.icon className="h-6 w-6" aria-hidden="true" />
              <span>{t(link.labelKey)}</span>
            </Link>
          );
        })}
      </nav>

      <div className="mt-auto flex shrink-0 items-center justify-between p-6">
        <Link
          href="/settings"
          title={t("settings")}
          aria-label={t("settings")}
          aria-current={isSettingsActive ? "page" : undefined}
          data-app-navigation-item="true"
          className={cn(
            "flex min-w-0 flex-1 items-center gap-4 px-2 transition-colors",
            isSettingsActive ? "text-gold" : "text-text hover:text-gold",
          )}
        >
          <CircleUser className="h-9 w-9 shrink-0" strokeWidth={1.5} aria-hidden="true" />
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-lg font-medium capitalize">
              {user?.email?.split("@")[0] || t("operator")}
            </span>
            <span className="truncate text-base capitalize">
              {activeTeam?.role || t("noStation")}
            </span>
          </div>
          <Settings className="h-5 w-5 shrink-0" aria-hidden="true" />
        </Link>
        <IconButton
          label={t("logout")}
          variant="ghost"
          tone="danger"
          size="sm"
          onClick={async () => {
            const { useAuthStore } = await import("@/store/auth");
            const { apiFetch } = await import("@/lib/api");
            // 1. Try to tell the server (don't await or care if it fails)
            apiFetch("/api/auth/logout", { method: "POST" }).catch(() => {});
            // 2. Clear store and let AuthGuard do the redirect
            useAuthStore.getState().logout();
          }}
          className="ml-4"
          title={t("logout")}
        >
          <LogOut className="h-5 w-5" aria-hidden="true" />
        </IconButton>
      </div>
    </aside>
  );
}
