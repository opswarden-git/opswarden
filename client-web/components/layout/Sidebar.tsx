"use client";

import { Link, usePathname } from "@/i18n/routing";
import { Settings } from "lucide-react";
import Image from "next/image";
import { useSearchParams } from "next/navigation";
import { useState } from "react";
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
import { useTeamScope } from "@/components/teams/TeamScope";
import { MemberAvatar, memberDisplayName } from "@/components/teams/MemberAvatar";
import { RoleChip } from "@/components/teams/RoleChip";
import { deriveCapabilities } from "@/lib/capabilities";
import { RailToggle } from "./RailToggle";
import { Dialog } from "@/components/ui/Dialog";
import { ProfilePanel } from "@/components/settings/ProfilePanel";
import { LanguagePanel } from "@/components/settings/LanguagePanel";
import { AccountDangerZone } from "@/components/settings/AccountDangerZone";

type ScopedTeam = ReturnType<typeof useTeamScope>["activeTeam"];

function NavLeaf({
  collapsed,
  leaf,
  pathname,
  searchParams,
  team,
}: {
  collapsed: boolean;
  leaf: NavigationLeaf;
  pathname: string;
  searchParams: Pick<URLSearchParams, "get">;
  team: ScopedTeam;
}) {
  const t = useTranslations("Sidebar");
  const isActive = isNavigationItemActive(pathname, leaf, searchParams);
  const count = leaf.countKey ? team?.[leaf.countKey] : undefined;
  const label = t(leaf.desktopLabelKey ?? leaf.labelKey);

  return (
    <Link
      href={leaf.href}
      title={collapsed ? label : undefined}
      aria-current={isActive ? "page" : undefined}
      data-app-navigation-item="true"
      className={cn(
        "group flex h-11 items-center gap-3 px-3 text-base transition-colors",
        collapsed && "justify-center px-0",
        isActive ? "text-gold font-medium" : "text-muted hover:text-gold",
      )}
    >
      <leaf.icon className="h-5 w-5 shrink-0" strokeWidth={1.8} aria-hidden="true" />
      <span className={cn("min-w-0 flex-1 truncate", collapsed && "sr-only")}>{label}</span>
      {count && !collapsed ? (
        <span className="shrink-0 text-sm tabular-nums opacity-60">{count}</span>
      ) : null}
    </Link>
  );
}

function NavGroup({
  collapsed,
  node,
  pathname,
  searchParams,
  team,
}: {
  collapsed: boolean;
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
        className={cn(
          "text-muted-2 px-3 pt-2 pb-1 text-[11px] font-semibold tracking-wider uppercase",
          collapsed && "sr-only",
        )}
      >
        {t(node.labelKey)}
      </h2>
      {node.children.map((child) => (
        <NavLeaf
          key={child.labelKey}
          collapsed={collapsed}
          leaf={child}
          pathname={pathname}
          searchParams={searchParams}
          team={team}
        />
      ))}
    </section>
  );
}

export function Sidebar({
  className,
  collapsed,
  onCollapsedChange,
}: {
  className?: string;
  collapsed: boolean;
  onCollapsedChange: (collapsed: boolean) => void;
}) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const t = useTranslations("Sidebar");
  const [isAccountOpen, setIsAccountOpen] = useState(false);
  const user = useAuthStore((state) => state.user);
  const { activeTeam, hrefFor } = useTeamScope();
  const canManageTeam = activeTeam
    ? deriveCapabilities(activeTeam.role).canManageAutomations
    : false;
  const tree = navigationTree(activeTeam?.team_id, canManageTeam);
  const isSettingsActive = isNavigationItemActive(pathname, settingsNavigationItem, searchParams);

  return (
    <aside
      className={cn(
        "border-border bg-panel relative flex shrink-0 flex-col border-r transition-[width] duration-200",
        collapsed ? "w-16" : "w-64",
        className,
      )}
      data-sidebar-collapsed={collapsed ? "true" : "false"}
    >
      <RailToggle
        className={cn(
          "top-1/2 -translate-y-1/2",
          collapsed ? "left-1/2 -translate-x-1/2" : "right-0",
        )}
        direction={collapsed ? "right" : "left"}
        label={t(collapsed ? "expandNavigation" : "collapseNavigation")}
        onClick={() => onCollapsedChange(!collapsed)}
      />
      <Link
        href={activeTeam ? hrefFor("overview") : "/teams"}
        title={collapsed ? t("logoWordmarkAlt") : undefined}
        className={cn(
          "flex h-16 w-full shrink-0 items-center gap-3 transition-opacity hover:opacity-80",
          collapsed ? "justify-center px-0" : "px-6",
        )}
      >
        <Image
          src="/assets/logo-icon.png"
          alt=""
          width={34}
          height={28}
          className="object-contain"
          priority
        />
        {!collapsed ? (
          <Image
            src="/assets/logo-text-light.png"
            alt={t("logoWordmarkAlt")}
            width={154}
            height={24}
            className="object-contain object-left"
            priority
          />
        ) : (
          <span className="sr-only">{t("logoWordmarkAlt")}</span>
        )}
      </Link>

      <nav
        aria-label={t("primaryNavigation")}
        className={cn("flex-1 space-y-4 overflow-y-auto py-4", collapsed ? "px-2" : "px-3")}
      >
        {tree.map((node) =>
          node.kind === "branch" ? (
            <NavGroup
              key={node.labelKey}
              collapsed={collapsed}
              node={node}
              pathname={pathname}
              searchParams={searchParams}
              team={activeTeam}
            />
          ) : (
            <NavLeaf
              key={node.labelKey}
              collapsed={collapsed}
              leaf={node}
              pathname={pathname}
              searchParams={searchParams}
              team={activeTeam}
            />
          ),
        )}
      </nav>

      <div className={cn("mt-auto shrink-0", collapsed ? "p-2" : "p-4")}>
        <Dialog
          open={isAccountOpen}
          onOpenChange={setIsAccountOpen}
          size="lg"
          title={t("account")}
          description={user?.email ?? t("operator")}
          closeLabel={t("close")}
          icon={<MemberAvatar email={user?.email || t("operator")} className="h-10 w-10 text-xs" />}
          bodyClassName="space-y-8"
          trigger={
            <button
              type="button"
              title={t("account")}
              aria-label={t("account")}
              aria-current={isSettingsActive ? "page" : undefined}
              data-app-navigation-item="true"
              className={cn(
                "group flex h-12 w-full min-w-0 items-center gap-2 rounded-md px-2 transition-colors",
                collapsed && "justify-center px-0",
                isSettingsActive ? "bg-panel text-text" : "text-text hover:bg-panel/60",
              )}
            >
              {!collapsed ? (
                <>
                  <MemberAvatar
                    email={user?.email || t("operator")}
                    className="h-8 w-8 text-[11px]"
                  />
                  <div className="flex min-w-0 flex-1 items-center gap-1.5">
                    <span className="truncate text-sm leading-5 font-medium">
                      {user?.email ? memberDisplayName(user.email) : t("operator")}
                    </span>
                    {activeTeam ? <RoleChip role={activeTeam.role} iconOnly /> : null}
                  </div>
                </>
              ) : null}
              <Settings
                className="text-muted group-hover:text-gold h-5 w-5 shrink-0 transition-colors"
                strokeWidth={1.8}
                aria-hidden="true"
              />
            </button>
          }
        >
          <ProfilePanel showIdentityHeader={false} showStationSetup={false} />
          <LanguagePanel />
          <AccountDangerZone />
        </Dialog>
      </div>
    </aside>
  );
}
