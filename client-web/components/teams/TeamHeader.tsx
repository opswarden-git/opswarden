"use client";

import { usePathname } from "@/i18n/routing";
import { useLocale, useTranslations } from "next-intl";
import type { Team } from "@/lib/queries/teams";
import { teamPath } from "@/lib/team-routing";
import { deriveCapabilities } from "@/lib/capabilities";
import { PageHeader } from "@/components/layout/PageHeader";
import { PageTabs } from "@/components/layout/PageTabs";
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
  const pathname = usePathname();
  const capabilities = deriveCapabilities(team.role);
  const tabs = [
    { section: "overview" as const, label: t("overview") },
    { section: "members" as const, label: t("members"), count: team.member_count },
    ...(capabilities.canManageAutomations
      ? [{ section: "automations" as const, label: t("automations") }]
      : []),
    { section: "settings" as const, label: t("settings") },
  ].map((tab) => {
    const href = teamPath(team.team_id, tab.section);
    return { ...tab, href, active: pathname === href || pathname.startsWith(`${href}/`) };
  });

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
      <PageTabs ariaLabel={t("teamNavigation")} tabs={tabs} />
    </>
  );
}
