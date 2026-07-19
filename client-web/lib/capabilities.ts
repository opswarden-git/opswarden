export type TeamRole = "observer" | "responder" | "manager";
export type IncidentStatus = "open" | "acknowledged" | "escalated" | "resolved";
export type IncidentTransition = Exclude<IncidentStatus, "open">;

export interface TeamCapabilities {
  canCreateIncident: boolean;
  canTransitionIncident: boolean;
  canAssignIncident: boolean;
  canDeleteIncident: boolean;
  canWriteTimeline: boolean;
  canSignalTyping: boolean;
  canReactTimeline: boolean;
  canCreateRelease: boolean;
  canProgressRelease: boolean;
  canLinkReleaseIncident: boolean;
  canCancelRelease: boolean;
  canManageMembers: boolean;
  canManageAutomations: boolean;
  canViewInvitationCode: boolean;
  canLeaveTeam: boolean;
  canDeleteTeam: boolean;
  canSendPrivateMessage: boolean;
}

const CAPABILITIES = {
  observer: {
    canCreateIncident: false,
    canTransitionIncident: false,
    canAssignIncident: false,
    canDeleteIncident: false,
    canWriteTimeline: false,
    canSignalTyping: false,
    canReactTimeline: true,
    canCreateRelease: false,
    canProgressRelease: false,
    canLinkReleaseIncident: false,
    canCancelRelease: false,
    canManageMembers: false,
    canManageAutomations: false,
    canViewInvitationCode: false,
    canLeaveTeam: true,
    canDeleteTeam: false,
    canSendPrivateMessage: true,
  },
  responder: {
    canCreateIncident: false,
    canTransitionIncident: true,
    canAssignIncident: false,
    canDeleteIncident: false,
    canWriteTimeline: true,
    canSignalTyping: true,
    canReactTimeline: true,
    canCreateRelease: false,
    canProgressRelease: true,
    canLinkReleaseIncident: true,
    canCancelRelease: false,
    canManageMembers: false,
    canManageAutomations: false,
    canViewInvitationCode: false,
    canLeaveTeam: true,
    canDeleteTeam: false,
    canSendPrivateMessage: true,
  },
  manager: {
    canCreateIncident: true,
    canTransitionIncident: true,
    canAssignIncident: true,
    canDeleteIncident: true,
    canWriteTimeline: true,
    canSignalTyping: true,
    canReactTimeline: true,
    canCreateRelease: true,
    canProgressRelease: true,
    canLinkReleaseIncident: true,
    canCancelRelease: true,
    canManageMembers: true,
    canManageAutomations: true,
    canViewInvitationCode: true,
    canLeaveTeam: false,
    canDeleteTeam: true,
    canSendPrivateMessage: true,
  },
} satisfies Record<TeamRole, TeamCapabilities>;

export function deriveCapabilities(role: TeamRole): TeamCapabilities {
  return CAPABILITIES[role];
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
