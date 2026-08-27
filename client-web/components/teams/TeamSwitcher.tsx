"use client";

import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { Check, ChevronDown, ListTree } from "lucide-react";
import { useTranslations } from "next-intl";
import { Link, usePathname } from "@/i18n/routing";
import { cn } from "@/lib/utils";
import { buttonClassNames } from "@/components/ui/Button";
import {
  MENU_SIDE_OFFSET,
  menuItemClassNames,
  menuSurfaceClassNames,
} from "@/components/ui/ActionMenu";
import { useTeamScope } from "./TeamScope";

export function TeamSwitcher({
  className,
  compact = false,
  presentation = "mobile",
}: {
  className?: string;
  compact?: boolean;
  presentation?: "mobile" | "breadcrumb";
}) {
  const t = useTranslations("TeamSwitcher");
  const pathname = usePathname();
  const { teams, activeTeam, isLoading, switchTeam } = useTeamScope();

  if (presentation === "breadcrumb") {
    return (
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild disabled={isLoading || !activeTeam}>
          <button
            type="button"
            aria-label={`${t("label")}: ${activeTeam?.name ?? t("noTeams")}`}
            className={cn(
              "text-muted hover:text-text focus-visible:ring-gold flex min-w-0 items-center gap-1 rounded-sm font-medium transition-colors focus-visible:ring-2 focus-visible:outline-none",
              className,
            )}
          >
            <span className="truncate">{activeTeam?.name ?? t("noTeams")}</span>
            <ChevronDown className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            align="start"
            sideOffset={MENU_SIDE_OFFSET}
            className={cn(menuSurfaceClassNames, "min-w-64")}
          >
            {teams.map((team) => {
              const current = team.team_id === activeTeam?.team_id;
              return (
                <DropdownMenu.CheckboxItem
                  key={team.team_id}
                  checked={current}
                  onCheckedChange={() => !current && switchTeam(team.team_id)}
                  className={cn(menuItemClassNames, "text-text")}
                >
                  <span className="min-w-0 flex-1 truncate">{team.name}</span>
                  <Check
                    className={cn("h-4 w-4 shrink-0", !current && "invisible")}
                    aria-hidden="true"
                  />
                </DropdownMenu.CheckboxItem>
              );
            })}
            <DropdownMenu.Separator className="bg-border my-1 h-px" />
            <DropdownMenu.Item asChild>
              <Link href="/teams" className={cn(menuItemClassNames, "text-text")}>
                {t("allTeams")}
              </Link>
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
    );
  }

  return (
    <div className={cn("flex min-w-0 items-end gap-2", className)}>
      <label className="min-w-0 flex-1">
        <span className={cn("text-muted mb-1 block text-xs font-medium", compact && "sr-only")}>
          {t("label")}
        </span>
        <select
          aria-label={t("label")}
          value={activeTeam?.team_id ?? ""}
          disabled={isLoading || teams.length === 0}
          onChange={(event) => switchTeam(event.target.value)}
          className={cn(
            "ow-input w-full min-w-0 rounded-md px-3 text-sm font-medium",
            compact ? "h-10" : "h-9",
          )}
        >
          {teams.length === 0 ? <option value="">{t("noTeams")}</option> : null}
          {teams.map((team) => (
            <option key={team.team_id} value={team.team_id} className="bg-bg text-text">
              {team.name}
            </option>
          ))}
        </select>
      </label>
      <Link
        href="/teams"
        aria-current={pathname === "/teams" ? "page" : undefined}
        data-app-navigation-item="true"
        className={buttonClassNames({
          size: compact ? "lg" : "md",
          className: compact ? "w-10 px-0" : "w-9 px-0",
        })}
        aria-label={t("directory")}
        title={t("directory")}
      >
        <ListTree className="h-4 w-4" aria-hidden="true" />
      </Link>
    </div>
  );
}
