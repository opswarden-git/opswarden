"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import {
  LayoutDashboard,
  ShieldAlert,
  Users,
  Settings,
  BotMessageSquare,
  CircleUser,
  LogOut,
} from "lucide-react";
import Image from "next/image";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/store/auth";
import { useTeams } from "@/lib/queries/teams";
import { useTranslations } from "next-intl";

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();
  const t = useTranslations("Sidebar");
  const user = useAuthStore((state) => state.user);
  const { data: teams } = useTeams();
  const primaryTeam = teams?.[0];

  const links = [
    { href: "/", icon: LayoutDashboard, label: t("dashboard") },
    { href: "/incidents", icon: ShieldAlert, label: t("incidents") },
    { href: "/teams", icon: Users, label: t("teams") },
    { href: "/ai", icon: BotMessageSquare, label: t("ai") },
  ];

  const isSettingsActive = pathname === "/settings" || pathname.startsWith("/settings/");

  return (
    <aside className={cn("glass flex w-80 flex-col", className)}>
      <Link
        href="/"
        className="flex h-28 w-full shrink-0 items-center justify-start gap-4 px-8 pt-4 transition-opacity hover:opacity-80"
      >
        <Image
          src="/assets/logo-icon.png"
          alt="Icon"
          width={32}
          height={32}
          className="h-8 w-auto object-contain"
          style={{ width: "auto" }}
        />
        <Image
          src="/assets/logo-text-light.png"
          alt="OpsWarden"
          width={220}
          height={44}
          className="h-7 w-auto object-contain object-left"
          style={{ width: "auto" }}
        />
      </Link>

      <nav className="flex-1 space-y-2 overflow-y-auto px-4 py-6">
        {links.map((link) => {
          const isActive = pathname === link.href || pathname.startsWith(link.href + "/");

          return (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "group flex items-center gap-4 px-4 py-3 text-lg transition-colors",
                isActive ? "text-gold font-medium" : "text-muted hover:text-gold",
              )}
            >
              <link.icon className="h-6 w-6" />
              <span>{link.label}</span>
            </Link>
          );
        })}
      </nav>

      <div className="mt-auto flex shrink-0 items-center justify-between p-6">
        <Link
          href="/settings"
          title={t("settings")}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-4 px-2 transition-colors",
            isSettingsActive ? "text-gold" : "text-text hover:text-gold",
          )}
        >
          <CircleUser className="h-9 w-9 shrink-0" strokeWidth={1.5} />
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-lg font-medium capitalize">
              {user?.email?.split("@")[0] || t("operator")}
            </span>
            <span className="truncate text-base capitalize">
              {primaryTeam?.role || t("noStation")}
            </span>
          </div>
          <Settings className="h-5 w-5 shrink-0" />
        </Link>
        <button
          onClick={async () => {
            const { useAuthStore } = await import("@/store/auth");
            const { apiFetch } = await import("@/lib/api");
            // 1. Try to tell the server (don't await or care if it fails)
            apiFetch("/api/auth/logout", { method: "POST" }).catch(() => {});
            // 2. Clear store and let AuthGuard do the redirect
            useAuthStore.getState().logout();
          }}
          className="text-muted ml-4 rounded-md p-2 transition-colors hover:text-red-500"
          title={t("logout")}
        >
          <LogOut className="h-5 w-5" />
        </button>
      </div>
    </aside>
  );
}
