"use client";

import { ListTree } from "lucide-react";
import { useTranslations } from "next-intl";
import { Link } from "@/i18n/routing";
import { cn } from "@/lib/utils";
import { buttonClassNames } from "@/components/ui/Button";
import { useTeamScope } from "./TeamScope";

export function TeamSwitcher({ className }: { className?: string }) {
  const t = useTranslations("TeamSwitcher");
  const { teams, activeTeam, isLoading, switchTeam } = useTeamScope();

  return (
    <div className={cn("flex min-w-0 items-end gap-2", className)}>
      <label className="min-w-0 flex-1 sm:w-64 sm:flex-none">
        <span className="text-muted mb-1 block text-xs font-medium">{t("label")}</span>
        <select
          aria-label={t("label")}
          value={activeTeam?.team_id ?? ""}
          disabled={isLoading || teams.length === 0}
          onChange={(event) => switchTeam(event.target.value)}
          className="ow-input h-9 w-full min-w-0 rounded-md px-3 text-sm"
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
        className={buttonClassNames({ size: "md", className: "w-9 px-0" })}
        aria-label={t("directory")}
        title={t("directory")}
      >
        <ListTree className="h-4 w-4" aria-hidden="true" />
      </Link>
    </div>
  );
}
