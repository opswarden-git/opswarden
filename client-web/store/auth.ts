import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AppLocale } from "@/i18n/locales";
import { useIncidentContextStore } from "@/store/incident-context";

export interface User {
  id: string;
  email?: string;
  locale: AppLocale;
}

interface AuthState {
  token: string | null;
  user: User | null;
  hasHydrated: boolean;

  setToken: (token: string) => void;
  setUser: (user: User) => void;
  setHasHydrated: (state: boolean) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      user: null,
      hasHydrated: false,

      setToken: (token) => set({ token }),
      setUser: (user) => set({ user }),
      setHasHydrated: (state) => set({ hasHydrated: state }),

      // Robust logout: reset authentication and operator-scoped navigation context.
      // The actual network call to invalidate the token is handled elsewhere.
      logout: () => {
        useIncidentContextStore.getState().clear();
        set({ token: null, user: null });
      },
    }),
    {
      name: "opswarden-auth-storage",
      // Exclude hasHydrated from persistence, it must start false on every load
      partialize: (state) => ({ token: state.token, user: state.user }),
      onRehydrateStorage: () => (state) => {
        // Called after hydration is complete
        state?.setHasHydrated(true);
      },
    },
  ),
);
