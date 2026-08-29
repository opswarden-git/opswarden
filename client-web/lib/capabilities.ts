import capabilityContract from "../../contracts/role-capabilities.json";

export type TeamRole = keyof typeof capabilityContract;

export type TeamCapabilities = (typeof capabilityContract)[TeamRole];

export function deriveCapabilities(role: TeamRole): TeamCapabilities {
  return capabilityContract[role];
}
