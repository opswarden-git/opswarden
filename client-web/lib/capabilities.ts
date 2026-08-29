import capabilityContract from "../../contracts/role-capabilities.json";

export type TeamRole = keyof typeof capabilityContract;
export type IncidentStatus = "open" | "acknowledged" | "escalated" | "resolved";
export type IncidentTransition = Exclude<IncidentStatus, "open">;

export type TeamCapabilities = (typeof capabilityContract)[TeamRole];

export function deriveCapabilities(role: TeamRole): TeamCapabilities {
  return capabilityContract[role];
}

export interface IncidentActions {
  canAssign: boolean;
  canDelete: boolean;
  canWriteTimeline: boolean;
  canReact: boolean;
  transitions: IncidentTransition[];
}

/** Role and state combined into the commands the current incident may expose. */
export function deriveIncidentActions(role: TeamRole, status: IncidentStatus): IncidentActions {
  const capabilities = deriveCapabilities(role);
  const transitions: IncidentTransition[] = capabilities.canTransitionIncident
    ? status === "open"
      ? ["acknowledged"]
      : status === "acknowledged"
        ? ["escalated", "resolved"]
        : status === "escalated"
          ? ["resolved"]
          : []
    : [];

  return {
    canAssign: capabilities.canAssignIncident && status !== "resolved",
    canDelete: capabilities.canDeleteIncident,
    canWriteTimeline: capabilities.canWriteTimeline,
    canReact: capabilities.canReactTimeline,
    transitions,
  };
}
