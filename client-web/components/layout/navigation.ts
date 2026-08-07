import {
  LayoutDashboard,
  PlugZap,
  Rocket,
  Settings,
  ShieldAlert,
  Users,
  Workflow,
} from "lucide-react";
import { parseTeamPath, teamPath, type TeamSection } from "@/lib/team-routing";
import type { AutomationView } from "@/lib/automation-routing";

type NavigationItem = {
  href: string;
  activeSections?: readonly TeamSection[];
  activePaths?: readonly string[];
  automationView?: AutomationView;
};

/**
 * Resolve the current product area once for every navigation surface.
 *
 * Team detail routes stay attached to their collection, while exact non-Team
 * paths (the Team directory and account settings) can opt into a stable root.
 */
export function isNavigationItemActive(
  pathname: string,
  item: NavigationItem,
  searchParams?: Pick<URLSearchParams, "get">,
) {
  const teamRoute = parseTeamPath(pathname);
  if (teamRoute) {
    if (!(item.activeSections?.includes(teamRoute.section) ?? false)) return false;
    if (!item.automationView) return true;

    const currentView = searchParams?.get("view") ?? "rules";
    if (currentView === "runs") return item.automationView === "rules";
    return currentView === item.automationView;
  }

  const activePaths = item.activePaths ?? [item.href];
  return activePaths.some((path) => pathname === path || pathname.startsWith(`${path}/`));
}

export type NavigationLeaf = NavigationItem & {
  icon: typeof ShieldAlert;
  labelKey: string;
  /** Which Team counter to show beside the label, when it is non-zero. */
  countKey?: "active_incident_count" | "active_release_count" | "member_count";
};

export type NavigationNode =
  | ({ kind: "leaf" } & NavigationLeaf)
  | {
      kind: "branch";
      labelKey: string;
      icon: typeof ShieldAlert;
      children: NavigationLeaf[];
    };

/**
 * Rules and Integrations are direct destinations even while they share the
 * stable /automations implementation route. Automation runs are contextual to
 * Rules and therefore never become a third primary destination.
 */
/** Operational history and Team configuration are Manager-only. */
export function navigationTree(teamId?: string, canManageTeam = false): NavigationNode[] {
  if (!teamId) {
    return [
      { kind: "leaf", href: "/teams", icon: Users, labelKey: "teams", activePaths: ["/teams"] },
    ];
  }

  return [
    {
      kind: "branch",
      labelKey: "operations",
      icon: LayoutDashboard,
      children: [
        {
          href: teamPath(teamId, "overview"),
          icon: LayoutDashboard,
          labelKey: "overview",
          activeSections: ["overview"] satisfies readonly TeamSection[],
        },
        {
          href: teamPath(teamId, "incidents"),
          icon: ShieldAlert,
          labelKey: "incidents",
          countKey: "active_incident_count",
          activeSections: ["incidents"] satisfies readonly TeamSection[],
        },
        {
          href: teamPath(teamId, "releases"),
          icon: Rocket,
          labelKey: "releases",
          countKey: "active_release_count",
          activeSections: ["releases"] satisfies readonly TeamSection[],
        },
        ...(canManageTeam
          ? [
              {
                href: teamPath(teamId, "automations"),
                icon: Workflow,
                labelKey: "rules",
                activeSections: ["automations"] satisfies readonly TeamSection[],
                automationView: "rules" as const,
              },
              {
                href: `${teamPath(teamId, "automations")}?view=connections`,
                icon: PlugZap,
                labelKey: "integrations",
                activeSections: ["automations"] satisfies readonly TeamSection[],
                automationView: "connections" as const,
              },
            ]
          : []),
      ],
    },
    {
      kind: "branch",
      labelKey: "manage",
      icon: Settings,
      children: [
        {
          href: teamPath(teamId, "members"),
          icon: Users,
          labelKey: "members",
          countKey: "member_count",
          activeSections: ["members"] satisfies readonly TeamSection[],
        },
        ...(canManageTeam
          ? [
              {
                href: teamPath(teamId, "settings"),
                icon: Settings,
                labelKey: "teamSettings",
                activeSections: ["settings"] satisfies readonly TeamSection[],
              },
            ]
          : []),
      ],
    },
  ];
}

/** Flat view of the same destinations, for surfaces that cannot nest. */
export function primaryNavigationItems(teamId?: string, canManageTeam = false): NavigationLeaf[] {
  return navigationTree(teamId, canManageTeam).flatMap((node) =>
    node.kind === "branch" ? node.children : [node],
  );
}

export const settingsNavigationItem = {
  href: "/settings",
  icon: Settings,
  labelKey: "settings",
  activePaths: ["/settings"],
} as const;
