import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface ActiveIncidentContext {
  incidentId: string;
  teamId: string;
  ownerId: string;
}

interface IncidentContextState {
  activeIncident: ActiveIncidentContext | null;
  hasHydrated: boolean;
  activate: (incident: ActiveIncidentContext) => void;
  clear: () => void;
  clearIfActive: (incidentId: string) => void;
  setHasHydrated: (state: boolean) => void;
}

/**
 * The selected incident is navigation context, not server-owned incident state.
 * Persisting only identifiers keeps the current record across navigation and a
 * reload without caching titles, severity or status that may have changed.
 */
export const useIncidentContextStore = create<IncidentContextState>()(
  persist(
    (set) => ({
      activeIncident: null,
      hasHydrated: false,
      activate: (activeIncident) => set({ activeIncident }),
      clear: () => set({ activeIncident: null }),
      clearIfActive: (incidentId) =>
        set((state) =>
          state.activeIncident?.incidentId === incidentId ? { activeIncident: null } : state,
        ),
      setHasHydrated: (hasHydrated) => set({ hasHydrated }),
    }),
    {
      name: "opswarden-incident-context",
      partialize: (state) => ({ activeIncident: state.activeIncident }),
      onRehydrateStorage: () => (state) => state?.setHasHydrated(true),
    },
  ),
);
