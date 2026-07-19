export const TEAM_SECTIONS = [
  "automations",
  "incidents",
  "members",
  "releases",
  "settings",
  "overview",
] as const;

export type TeamSection = (typeof TEAM_SECTIONS)[number];

export type TeamRoute = {
  teamId: string;
  section: TeamSection;
  resourceId?: string;
};

const TEAM_ROUTE_PATTERN = new RegExp(
  `^/teams/([^/]+)/(${TEAM_SECTIONS.join("|")})(?:/([^/]+))?/?$`,
);

export function teamPath(teamId: string, section: TeamSection = "incidents", resourceId?: string) {
  const base = `/teams/${encodeURIComponent(teamId)}/${section}`;
  return resourceId ? `${base}/${encodeURIComponent(resourceId)}` : base;
}

export function parseTeamPath(pathname: string): TeamRoute | null {
  const match = pathname.match(TEAM_ROUTE_PATTERN);
  if (!match) return null;

  return {
    teamId: decodeURIComponent(match[1]),
    section: match[2] as TeamSection,
    resourceId: match[3] ? decodeURIComponent(match[3]) : undefined,
  };
}

/**
 * Change the Team while preserving the current product area. Resource IDs are
 * deliberately dropped because an Incident or Release belongs to one Team.
 */
export function pathForTeamSwitch(pathname: string, nextTeamId: string) {
  const current = parseTeamPath(pathname);
  return teamPath(nextTeamId, current?.section ?? "incidents");
}
