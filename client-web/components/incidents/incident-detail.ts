import type { IncidentTransition } from "@/lib/capabilities";

export interface IncidentHeaderActions {
  primary: IncidentTransition | null;
  secondary: IncidentTransition | null;
}

/**
 * Turns the domain-authorized transitions into a predictable page hierarchy.
 * The server remains authoritative; this function only decides presentation.
 */
export function deriveIncidentHeaderActions(
  transitions: IncidentTransition[],
): IncidentHeaderActions {
  if (transitions.includes("acknowledged")) {
    return { primary: "acknowledged", secondary: null };
  }

  if (transitions.includes("escalated")) {
    return {
      primary: "escalated",
      secondary: transitions.includes("resolved") ? "resolved" : null,
    };
  }

  if (transitions.includes("resolved")) {
    return { primary: "resolved", secondary: null };
  }

  return { primary: null, secondary: null };
}
