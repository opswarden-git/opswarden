"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/routing";
import { ChevronDown, ChevronRight, CircleUser, LogOut, Settings } from "lucide-react";
import Image from "next/image";
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
  team,
  nested = false,
}: {
  leaf: NavigationLeaf;
  pathname: string;
  team: ScopedTeam;
  nested?: boolean;
}) {
  const t = useTranslations("Sidebar");
  const isActive = isNavigationItemActive(pathname, leaf);
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
        "relative flex h-9 items-center gap-3 rounded-md pr-3 text-sm transition-colors",
        nested ? "pl-9" : "pl-3",
        isActive
          ? "text-text bg-panel before:bg-gold font-medium before:absolute before:inset-y-2 before:left-0 before:w-0.5 before:rounded-full"
          : "text-muted hover:bg-panel/60 hover:text-text",
      )}
    >
      {nested ? null : (
        <leaf.icon className="h-4 w-4 shrink-0" strokeWidth={1.8} aria-hidden="true" />
      )}
      <span className="min-w-0 flex-1 truncate">{t(leaf.labelKey)}</span>
      {count ? <span className="text-muted-2 shrink-0 text-xs tabular-nums">{count}</span> : null}
    </Link>
  );
}

function NavBranch({
  node,
  pathname,
  team,
}: {
  node: Extract<NavigationNode, { kind: "branch" }>;
  pathname: string;
  team: ScopedTeam;
}) {
  const t = useTranslations("Sidebar");
  const holdsCurrentPage = node.children.some((child) => isNavigationItemActive(pathname, child));
  /*
   * Derived, not synchronised. The branch opens itself whenever it holds the
   * current page — so the operator never has to discover where they already
   * are — and a deliberate toggle wins until the route moves again. Mirroring
   * the route into state through an effect would re-render on every navigation
   * for a value that is already known during render.
   */
  const [toggled, setToggled] = React.useState<boolean | null>(null);
  const open = toggled ?? holdsCurrentPage;

  return (
    <div>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setToggled(!open)}
        className={cn(
          "flex h-9 w-full items-center gap-3 rounded-md px-3 text-sm transition-colors",
          holdsCurrentPage
            ? "text-text font-medium"
            : "text-muted hover:bg-panel/60 hover:text-text",
        )}
      >
        <node.icon className="h-4 w-4 shrink-0" strokeWidth={1.8} aria-hidden="true" />
        <span className="min-w-0 flex-1 truncate text-left">{t(node.labelKey)}</span>
        {open ? (
          <ChevronDown className="text-muted-2 h-4 w-4 shrink-0" aria-hidden="true" />
        ) : (
          <ChevronRight className="text-muted-2 h-4 w-4 shrink-0" aria-hidden="true" />
        )}
      </button>
      {open ? (
        <div className="border-border ml-5 space-y-1 border-l pl-0">
          {node.children.map((child) => (
            <NavLeaf key={child.labelKey} leaf={child} pathname={pathname} team={team} nested />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function Sidebar({ className }: { className?: string }) {
  const pathname = usePathname();
  const t = useTranslations("Sidebar");
  const user = useAuthStore((state) => state.user);
  const { activeTeam, hrefFor } = useTeamScope();
  const canViewActivity = activeTeam
    ? deriveCapabilities(activeTeam.role).canManageAutomations
    : false;
  const tree = navigationTree(activeTeam?.team_id, canViewActivity);
  const isSettingsActive = isNavigationItemActive(pathname, settingsNavigationItem);

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
        className="flex-1 space-y-1 overflow-y-auto px-3 py-4"
      >
        {tree.map((node) =>
          node.kind === "branch" ? (
            <NavBranch key={node.labelKey} node={node} pathname={pathname} team={activeTeam} />
          ) : (
            <NavLeaf key={node.labelKey} leaf={node} pathname={pathname} team={activeTeam} />
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
