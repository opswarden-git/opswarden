"use client";

import { useLocale, useTranslations } from "next-intl";
import type { Team } from "@/lib/queries/teams";
import { PageHeader } from "@/components/layout/PageHeader";
import { RoleChip } from "./RoleChip";
import { TeamSwitcher } from "./TeamSwitcher";

export function TeamHeader({
  team,
  showTeamSwitcher = false,
}: {
  team: Team;
  showTeamSwitcher?: boolean;
}) {
  const t = useTranslations("Teams");
  const locale = useLocale();
  return (
    <>
      <PageHeader
        title={team.name}
        description={t("workspaceDescription")}
        metadata={
          <div className="flex items-center gap-2">
            <RoleChip role={team.role} />
            <span>
              {t("createdOn", {
                date: new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                  new Date(team.created_at),
                ),
              })}
            </span>
          </div>
        }
        actions={showTeamSwitcher ? <TeamSwitcher className="w-full sm:w-64" /> : undefined}
      />
    </>
  );
}
