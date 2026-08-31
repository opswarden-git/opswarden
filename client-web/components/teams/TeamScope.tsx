"use client";

import React from "react";
import { deriveCapabilities, type TeamCapabilities, type TeamRole } from "@/lib/capabilities";
import * as routing from "@/i18n/routing";
import { useTeams, type Team } from "@/lib/queries/teams";
import { parseTeamPath, pathForTeamSwitch, teamPath, type TeamSection } from "@/lib/team-routing";

export type TeamScopeValue = {
  teams: Team[];
  activeTeam?: Team;
  role?: TeamRole;
  capabilities: TeamCapabilities;
  isLoading: boolean;
  error: Error | null;
  isValidScope: boolean;
  switchTeam: (teamId: string) => void;
  hrefFor: (section: TeamSection, resourceId?: string) => string;
};

const TeamScopeContext = React.createContext<TeamScopeValue | null>(null);
const NO_TEAMS: Team[] = [];

export function TeamScopeProvider({ children }: { children: React.ReactNode }) {
  const pathname = routing.usePathname?.() ?? "";
  const router = routing.useRouter?.() ?? { push: () => {}, replace: () => {} };
  const { data, isLoading, error } = useTeams();
  const teams = data ?? NO_TEAMS;
  const route = React.useMemo(() => parseTeamPath(pathname), [pathname]);
  const routeTeam = route ? teams.find((team) => team.team_id === route.teamId) : undefined;
  const activeTeam = route ? routeTeam : teams[0];
  const role = activeTeam?.role;
  const capabilities = React.useMemo(() => deriveCapabilities(role ?? "observer"), [role]);
  const isValidScope = Boolean(!route || routeTeam);

  React.useEffect(() => {
    if (isLoading || error || !route || routeTeam) return;

    const fallback = teams[0];
    router.replace(fallback ? teamPath(fallback.team_id, route.section) : "/teams");
  }, [error, isLoading, route, routeTeam, router, teams]);

  const value = React.useMemo<TeamScopeValue>(
    () => ({
      teams,
      activeTeam,
      role,
      capabilities,
      isLoading,
      error: (error as Error) ?? null,
      isValidScope,
      switchTeam: (teamId) => router.push(pathForTeamSwitch(pathname, teamId)),
      hrefFor: (section, resourceId) =>
        activeTeam ? teamPath(activeTeam.team_id, section, resourceId) : "/teams",
    }),
    [activeTeam, capabilities, error, isLoading, isValidScope, pathname, role, router, teams],
  );

  return <TeamScopeContext.Provider value={value}>{children}</TeamScopeContext.Provider>;
}

export function useTeamScope(): TeamScopeValue {
  const value = React.useContext(TeamScopeContext);
  if (value) return value;

  return useFallbackTeamScope();
}

function useFallbackTeamScope(): TeamScopeValue {
  const teamsQuery = useTeams();
  const rawPathname = routing.usePathname?.() ?? "";
  const router = routing.useRouter?.() ?? { push: () => {}, replace: () => {} };
  const pathname = rawPathname ?? "";

  const route = React.useMemo(() => parseTeamPath(pathname), [pathname]);
  const fallbackTeams = teamsQuery.data ?? NO_TEAMS;
  const fallbackActiveTeam = route
    ? fallbackTeams.find((team) => team.team_id === route.teamId)
    : fallbackTeams[0];
  const fallbackRole = fallbackActiveTeam?.role;
  const fallbackCapabilities = React.useMemo(
    () => deriveCapabilities(fallbackRole ?? "observer"),
    [fallbackRole],
  );

  return {
    teams: fallbackTeams,
    activeTeam: fallbackActiveTeam,
    role: fallbackRole,
    capabilities: fallbackCapabilities,
    isLoading: teamsQuery.isLoading,
    error: (teamsQuery.error as Error) ?? null,
    isValidScope: Boolean(!route || fallbackActiveTeam),
    switchTeam: (teamId) => router.push(pathForTeamSwitch(pathname, teamId)),
    hrefFor: (section, resourceId) =>
      fallbackActiveTeam ? teamPath(fallbackActiveTeam.team_id, section, resourceId) : "/teams",
  };
}
