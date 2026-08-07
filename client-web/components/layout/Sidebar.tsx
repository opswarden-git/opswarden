"use client";

import { Link, usePathname } from "@/i18n/routing";
import { CircleUser, LogOut, Settings } from "lucide-react";
import Image from "next/image";
import { useSearchParams } from "next/navigation";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/store/auth";
import { useTranslations } from "next-intl";
import {
  isNavigationItemActive,
  navigationTree,
  settingsNavigationItem,
  type NavigationLeaf,
  type NavigationNode,
} from "./navigation";
import { IconButton } from "@/components/ui/Button";
import { useTeamScope } from "@/components/teams/TeamScope";
import { TeamSwitcher } from "@/components/teams/TeamSwitcher";
import { deriveCapabilities } from "@/lib/capabilities";

type ScopedTeam = ReturnType<typeof useTeamScope>["activeTeam"];

function NavLeaf({
  leaf,
  pathname,
  searchParams,
  team,
}: {
  leaf: NavigationLeaf;
  pathname: string;
  searchParams: Pick<URLSearchParams, "get">;
  team: ScopedTeam;
}) {
  const t = useTranslations("Sidebar");
  const isActive = isNavigationItemActive(pathname, leaf, searchParams);
  const count = leaf.countKey ? team?.[leaf.countKey] : undefined;

  return (
    <Link
      href={leaf.href}
      aria-current={isActive ? "page" : undefined}
      data-app-navigation-item="true"
      className={cn(
        // The active mark is a rule plus the text weight, never a gold fill:
        // gold is the primary-action colour, and spending it on "you are here"
        // leaves nothing to say "act here".
        "relative flex h-9 items-center gap-3 rounded-md px-3 text-sm transition-colors",
        isActive
          ? "text-text bg-panel before:bg-gold font-medium before:absolute before:inset-y-2 before:left-0 before:w-0.5 before:rounded-full"
          : "text-muted hover:bg-panel/60 hover:text-text",
      )}
    >
      <leaf.icon className="h-4 w-4 shrink-0" strokeWidth={1.8} aria-hidden="true" />
      <span className="min-w-0 flex-1 truncate">{t(leaf.labelKey)}</span>
      {count ? <span className="text-muted-2 shrink-0 text-xs tabular-nums">{count}</span> : null}
    </Link>
  );
}

function NavGroup({
  node,
  pathname,
  searchParams,
  team,
}: {
  node: Extract<NavigationNode, { kind: "branch" }>;
  pathname: string;
  searchParams: Pick<URLSearchParams, "get">;
  team: ScopedTeam;
}) {
  const t = useTranslations("Sidebar");

  return (
    <section aria-labelledby={`nav-${node.labelKey}`} className="space-y-1">
      <h2
        id={`nav-${node.labelKey}`}
        className="text-muted-2 px-3 pt-2 pb-1 text-[11px] font-semibold tracking-wider uppercase"
      >
        {t(node.labelKey)}
      </h2>
      {node.children.map((child) => (
        <NavLeaf
          key={child.labelKey}
          leaf={child}
          pathname={pathname}
          searchParams={searchParams}
          team={team}
        />
      ))}
    </section>
  );
}

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const t = useTranslations("Sidebar");
  const user = useAuthStore((state) => state.user);
  const { activeTeam, hrefFor } = useTeamScope();
  const canManageTeam = activeTeam
    ? deriveCapabilities(activeTeam.role).canManageAutomations
    : false;
  const tree = navigationTree(activeTeam?.team_id, canManageTeam);
  const isSettingsActive = isNavigationItemActive(pathname, settingsNavigationItem, searchParams);

  return (
    <aside className={cn("border-border bg-bg-2 flex w-64 shrink-0 flex-col border-r", className)}>
      <Link
        href={activeTeam ? hrefFor("incidents") : "/teams"}
        className="border-border flex h-20 w-full shrink-0 items-center gap-3 border-b px-5 transition-opacity hover:opacity-80"
      >
        <Image
          src="/assets/logo-icon.png"
          alt=""
          width={34}
          height={28}
          className="object-contain"
          priority
        />
        <Image
          src="/assets/logo-text-light.png"
          alt={t("logoWordmarkAlt")}
          width={154}
          height={24}
          className="object-contain object-left"
          priority
        />
      </Link>

      <div className="border-border border-b px-4 py-4">
        <TeamSwitcher className="w-full" />
      </div>

      <nav
        aria-label={t("primaryNavigation")}
        className="flex-1 space-y-4 overflow-y-auto px-3 py-4"
      >
        {tree.map((node) =>
          node.kind === "branch" ? (
            <NavGroup
              key={node.labelKey}
              node={node}
              pathname={pathname}
              searchParams={searchParams}
              team={activeTeam}
            />
          ) : (
            <NavLeaf
              key={node.labelKey}
              leaf={node}
              pathname={pathname}
              searchParams={searchParams}
              team={activeTeam}
            />
          ),
        )}
      </nav>

      <div className="border-border mt-auto flex shrink-0 items-center gap-2 border-t p-4">
        <Link
          href="/settings"
          title={t("settings")}
          aria-label={t("settings")}
          aria-current={isSettingsActive ? "page" : undefined}
          data-app-navigation-item="true"
          className={cn(
            "flex min-w-0 flex-1 items-center gap-3 rounded-md p-2 transition-colors",
            isSettingsActive ? "bg-panel text-text" : "text-text hover:bg-panel/60",
          )}
        >
          <span className="bg-panel-2 flex h-9 w-9 shrink-0 items-center justify-center rounded-full">
            <CircleUser className="h-5 w-5" strokeWidth={1.7} aria-hidden="true" />
          </span>
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-sm font-medium">
              {user?.email?.split("@")[0] || t("operator")}
            </span>
            <span className="text-muted truncate text-xs capitalize">
              {activeTeam?.role || t("noStation")}
            </span>
          </div>
          <Settings className="text-muted h-4 w-4 shrink-0" aria-hidden="true" />
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
          title={t("logout")}
        >
          <LogOut className="h-5 w-5" aria-hidden="true" />
        </IconButton>
      </div>
    </aside>
  );
}
