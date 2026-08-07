import {
  Activity,
  LayoutDashboard,
  Rocket,
  Settings,
  ShieldAlert,
  SlidersHorizontal,
  Users,
} from "lucide-react";
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
 * One tree, shaped the way the incident-response tools shape theirs.
 *
 * IRIS puts `Dashboard` and `Overview` flat and above every section, then
 * groups administration under `Manage`. TheHive keeps `My tasks`, `Alerts`,
 * `Dashboards` and `Search` flat, and folds Organisations, Profiles, Custom
 * fields and Taxonomies under `Admin`. Both draw the same line: what you do
 * during a shift is never nested, what you configure between shifts always is.
 *
 * So the landing page sits above the branch rather than inside it, and the
 * branch is named after the action it covers rather than after a container.
 * The Team sections used to be folded behind one entry and re-exposed as a tab
 * strip inside the page, which meant the sidebar said where you were and the
 * tabs said something else. A branch that opens in place says both at once.
 */
/** Activity is Manager-only because the API that lists automation runs is too. */
export function navigationTree(teamId?: string, canViewActivity = false): NavigationNode[] {
  if (!teamId) {
    return [
      { kind: "leaf", href: "/teams", icon: Users, labelKey: "teams", activePaths: ["/teams"] },
    ];
  }

  return [
    {
      kind: "leaf",
      href: teamPath(teamId, "overview"),
      icon: LayoutDashboard,
      labelKey: "overview",
      activeSections: ["overview"] satisfies readonly TeamSection[],
    },
    {
      kind: "leaf",
      href: teamPath(teamId, "incidents"),
      icon: ShieldAlert,
      labelKey: "incidents",
      countKey: "active_incident_count",
      activeSections: ["incidents"] satisfies readonly TeamSection[],
    },
    {
      kind: "leaf",
      href: teamPath(teamId, "releases"),
      icon: Rocket,
      labelKey: "releases",
      countKey: "active_release_count",
      activeSections: ["releases"] satisfies readonly TeamSection[],
    },
    ...(canViewActivity
      ? [
          {
            kind: "leaf" as const,
            href: teamPath(teamId, "activity"),
            icon: Activity,
            labelKey: "activity",
            activeSections: ["activity"] satisfies readonly TeamSection[],
          },
        ]
      : []),
    {
      kind: "branch",
      labelKey: "manage",
      icon: SlidersHorizontal,
      children: [
        {
          href: teamPath(teamId, "members"),
          icon: Users,
          labelKey: "members",
          countKey: "member_count",
          activeSections: ["members"] satisfies readonly TeamSection[],
        },
        {
          href: teamPath(teamId, "automations"),
          icon: SlidersHorizontal,
          labelKey: "automations",
          activeSections: ["automations"] satisfies readonly TeamSection[],
        },
        {
          href: teamPath(teamId, "settings"),
          icon: Settings,
          labelKey: "teamSettings",
          activeSections: ["settings"] satisfies readonly TeamSection[],
        },
      ],
    },
  ];
}

/** Flat view of the same destinations, for surfaces that cannot nest. */
export function primaryNavigationItems(teamId?: string, canViewActivity = false): NavigationLeaf[] {
  return navigationTree(teamId, canViewActivity).flatMap((node) =>
    node.kind === "branch" ? node.children : [node],
  );
}

export const settingsNavigationItem = {
  href: "/settings",
  icon: Settings,
  labelKey: "settings",
  activePaths: ["/settings"],
} as const;
