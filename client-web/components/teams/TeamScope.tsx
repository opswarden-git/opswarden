"use client";

import React from "react";
import { usePathname, useRouter } from "@/i18n/routing";
import { useTeams, type Team } from "@/lib/queries/teams";
import { parseTeamPath, pathForTeamSwitch, teamPath, type TeamSection } from "@/lib/team-routing";

type TeamScopeValue = {
  teams: Team[];
  activeTeam?: Team;
  isLoading: boolean;
  switchTeam: (teamId: string) => void;
  hrefFor: (section: TeamSection, resourceId?: string) => string;
};

const TeamScopeContext = React.createContext<TeamScopeValue | null>(null);
const NO_TEAMS: Team[] = [];

export function TeamScopeProvider({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const { data, isLoading } = useTeams();
  const teams = data ?? NO_TEAMS;
  const route = React.useMemo(() => parseTeamPath(pathname), [pathname]);
  const routeTeam = route ? teams.find((team) => team.team_id === route.teamId) : undefined;
  const activeTeam = route ? routeTeam : teams[0];

  React.useEffect(() => {
    if (isLoading || !route || routeTeam) return;

    const fallback = teams[0];
    router.replace(fallback ? teamPath(fallback.team_id, route.section) : "/teams");
  }, [isLoading, route, routeTeam, router, teams]);

  const value = React.useMemo<TeamScopeValue>(
    () => ({
      teams,
      activeTeam,
      isLoading,
      switchTeam: (teamId) => router.push(pathForTeamSwitch(pathname, teamId)),
      hrefFor: (section, resourceId) =>
        activeTeam ? teamPath(activeTeam.team_id, section, resourceId) : "/teams",
    }),
    [activeTeam, isLoading, pathname, router, teams],
  );

  return <TeamScopeContext.Provider value={value}>{children}</TeamScopeContext.Provider>;
}

export function useTeamScope() {
  const value = React.useContext(TeamScopeContext);
  if (!value) throw new Error("useTeamScope must be used inside TeamScopeProvider");
  return value;
}
