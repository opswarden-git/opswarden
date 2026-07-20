import { Rocket, Settings, ShieldAlert, Users } from "lucide-react";
import { parseTeamPath, teamPath, type TeamSection } from "@/lib/team-routing";

type NavigationItem = {
  href: string;
  activeSections?: readonly TeamSection[];
  activePaths?: readonly string[];
};

/**
 * Resolve the current product area once for every navigation surface.
 *
 * Team detail routes stay attached to their collection, while exact non-Team
 * paths (the Team directory and account settings) can opt into a stable root.
 */
export function isNavigationItemActive(pathname: string, item: NavigationItem) {
  const teamRoute = parseTeamPath(pathname);
  if (teamRoute) return item.activeSections?.includes(teamRoute.section) ?? false;

  const activePaths = item.activePaths ?? [item.href];
  return activePaths.some((path) => pathname === path || pathname.startsWith(`${path}/`));
}

export function primaryNavigationItems(teamId?: string) {
  const teamItems = teamId
    ? [
        {
          href: teamPath(teamId, "incidents"),
          icon: ShieldAlert,
          labelKey: "incidents" as const,
          activeSections: ["incidents"] satisfies readonly TeamSection[],
        },
        {
          href: teamPath(teamId, "releases"),
          icon: Rocket,
          labelKey: "releases" as const,
          activeSections: ["releases"] satisfies readonly TeamSection[],
        },
      ]
    : [];

  return [
    ...teamItems,
    {
      href: teamId ? teamPath(teamId, "overview") : "/teams",
      icon: Users,
      labelKey: "teams" as const,
      activeSections: [
        "overview",
        "members",
        "automations",
        "settings",
      ] satisfies readonly TeamSection[],
      activePaths: ["/teams"],
    },
  ];
}

export const settingsNavigationItem = {
  href: "/settings",
  icon: Settings,
  labelKey: "settings",
  activePaths: ["/settings"],
} as const;
