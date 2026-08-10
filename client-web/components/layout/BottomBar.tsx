"use client";

import { useState } from "react";
import { CircleUser, MoreHorizontal } from "lucide-react";
import { useSearchParams } from "next/navigation";
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
import { Dialog, DialogClose } from "@/components/ui/Dialog";

const MOBILE_PRIMARY_LABELS = new Set(["overview", "incidents", "releases"]);

const itemClassName = (isActive: boolean) =>
  cn(
    "relative flex h-full min-w-0 flex-1 flex-col items-center justify-center gap-1 transition-colors",
    isActive
      ? "text-gold after:bg-gold after:absolute after:top-0 after:h-0.5 after:w-8 after:rounded-full"
      : "text-muted hover:text-text",
  );

export function BottomBar({ className }: { className?: string }) {
  const [isMoreOpen, setIsMoreOpen] = useState(false);
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const t = useTranslations("Sidebar");
  const { activeTeam } = useTeamScope();
  const canManageTeam = activeTeam
    ? deriveCapabilities(activeTeam.role).canManageAutomations
    : false;
  const teamLinks = primaryNavigationItems(activeTeam?.team_id, canManageTeam);
  const primaryLinks = activeTeam
    ? teamLinks.filter((item) => MOBILE_PRIMARY_LABELS.has(item.labelKey))
    : teamLinks;
  const moreLinks = activeTeam
    ? [
        ...teamLinks.filter((item) => !MOBILE_PRIMARY_LABELS.has(item.labelKey)),
        settingsNavigationItem,
      ]
    : [settingsNavigationItem];
  const isMoreActive = moreLinks.some((item) =>
    isNavigationItemActive(pathname, item, searchParams),
  );

  return (
    <nav
      aria-label={t("mobileNavigation")}
      className={cn(
        "border-border bg-bg/95 fixed right-0 bottom-0 left-0 z-50 flex h-16 items-center justify-around border-t px-2 shadow-[0_-8px_24px_rgb(0_0_0/0.18)] backdrop-blur-md",
        className,
      )}
    >
      {primaryLinks.map((link) => {
        const isActive = isNavigationItemActive(pathname, link, searchParams);

        return (
          <Link
            key={link.labelKey}
            href={link.href}
            aria-current={isActive ? "page" : undefined}
            data-app-navigation-item="true"
            className={itemClassName(isActive)}
          >
            <link.icon className="h-5 w-5" strokeWidth={1.8} aria-hidden="true" />
            <span className="w-full truncate px-0.5 text-center text-[10px] font-medium">
              {t(link.labelKey)}
            </span>
          </Link>
        );
      })}

      <Dialog
        open={isMoreOpen}
        onOpenChange={setIsMoreOpen}
        variant="sheet"
        title={t("more")}
        description={t("mobileNavigation")}
        closeLabel={t("close")}
        bodyClassName="p-3 pb-4"
        trigger={
          <button
            type="button"
            aria-current={isMoreActive ? "page" : undefined}
            data-app-navigation-item="true"
            className={itemClassName(isMoreActive)}
          >
            <MoreHorizontal className="h-5 w-5" strokeWidth={1.8} aria-hidden="true" />
            <span className="w-full truncate px-0.5 text-center text-[10px] font-medium">
              {t("more")}
            </span>
          </button>
        }
      >
        <nav aria-label={t("mobileNavigation")} className="grid gap-1">
          {moreLinks.map((link) => {
            const isActive = isNavigationItemActive(pathname, link, searchParams);
            const Icon = link.labelKey === settingsNavigationItem.labelKey ? CircleUser : link.icon;

            return (
              <DialogClose key={link.labelKey}>
                <Link
                  href={link.href}
                  aria-current={isActive ? "page" : undefined}
                  data-app-navigation-item="true"
                  className={cn(
                    "flex min-h-12 items-center gap-3 rounded-md px-3 text-sm transition-colors",
                    isActive
                      ? "bg-panel text-text font-medium"
                      : "text-muted hover:bg-panel/60 hover:text-text",
                  )}
                >
                  <Icon className="h-5 w-5 shrink-0" strokeWidth={1.8} aria-hidden="true" />
                  <span>{t(link.labelKey)}</span>
                </Link>
              </DialogClose>
            );
          })}
        </nav>
      </Dialog>
    </nav>
  );
}
