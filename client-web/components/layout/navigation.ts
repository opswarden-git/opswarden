import { Rocket, Settings, ShieldAlert, Users } from "lucide-react";
import { teamPath } from "@/lib/team-routing";

export function primaryNavigationItems(teamId?: string) {
  const teamItems = teamId
    ? [
        {
          href: teamPath(teamId, "incidents"),
          icon: ShieldAlert,
          labelKey: "incidents" as const,
          activeSections: ["incidents"] as const,
        },
        {
          href: teamPath(teamId, "releases"),
          icon: Rocket,
          labelKey: "releases" as const,
          activeSections: ["releases"] as const,
        },
      ]
    : [];

  return [
    ...teamItems,
    {
      href: teamId ? teamPath(teamId, "overview") : "/teams",
      icon: Users,
      labelKey: "teams" as const,
      activeSections: ["overview", "members", "automations", "settings"] as const,
    },
  ];
}

export const settingsNavigationItem = {
  href: "/settings",
  icon: Settings,
  labelKey: "settings",
} as const;
