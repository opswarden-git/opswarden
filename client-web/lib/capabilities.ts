import capabilityContract from "../../contracts/role-capabilities.json";

export type TeamRole = keyof typeof capabilityContract;
export type AssignableTeamRole = Exclude<TeamRole, "manager">;
export const TEAM_ROLES = Object.keys(capabilityContract) as TeamRole[];

export type TeamCapabilities = (typeof capabilityContract)[TeamRole];

export function deriveCapabilities(role?: string): TeamCapabilities {
  if (role && role in capabilityContract) {
    return capabilityContract[role as TeamRole];
  }
  return capabilityContract.responder;
}
