export const INCIDENT_STATUSES = ["open", "acknowledged", "escalated", "resolved"] as const;
export const INCIDENT_SEVERITIES = ["low", "medium", "high", "critical"] as const;

export type IncidentStatus = (typeof INCIDENT_STATUSES)[number];
export type IncidentTransition = Exclude<IncidentStatus, "open">;
export type IncidentSeverity = (typeof INCIDENT_SEVERITIES)[number];

export function isIncidentStatus(value: string): value is IncidentStatus {
  return (INCIDENT_STATUSES as readonly string[]).includes(value);
}

export function isIncidentSeverity(value: string): value is IncidentSeverity {
  return (INCIDENT_SEVERITIES as readonly string[]).includes(value);
}
