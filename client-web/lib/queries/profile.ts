import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppLocale } from "@/i18n/locales";
import { apiFetch } from "@/lib/api";
import { useAuthStore, type User } from "@/store/auth";

export const profileQueryKey = ["profile"] as const;

async function readProfile(): Promise<User> {
  const response = await apiFetch("/api/me");
  if (!response.ok) throw new Error("profile_load_failed");
  return response.json();
}

export function useProfile(enabled = true) {
  return useQuery({
    queryKey: profileQueryKey,
    queryFn: readProfile,
    enabled,
  });
}

export function useUpdateLocale() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (locale: AppLocale): Promise<User> => {
      const response = await apiFetch("/api/me/locale", {
        method: "PUT",
        body: JSON.stringify({ locale }),
      });
      if (!response.ok) throw new Error("locale_update_failed");
      return response.json();
    },
    onSuccess: (profile) => {
      useAuthStore.getState().setUser(profile);
      queryClient.setQueryData(profileQueryKey, profile);
    },
  });
}
