import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

/** Connection status per third-party service. Secrets are never returned. */
export interface ServiceConnections {
  github: { connected: boolean };
}

export function useServiceConnections() {
  return useQuery<ServiceConnections>({
    queryKey: ["service-connections"],
    queryFn: async () => {
      const res = await apiFetch("/api/service-connections");
      if (!res.ok) throw new Error("Failed to load service connections");
      return res.json();
    },
  });
}

/**
 * Store the GitHub inbound-webhook secret server-side (encrypted vault). On error
 * the thrown message is the backend's stable error `code` (e.g.
 * `invalid_service_secret`) so the UI can map it through `errors.<code>` i18n.
 */
export function useConnectGithub() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (webhook_secret: string) => {
      const res = await apiFetch("/api/service-connections/github", {
        method: "PUT",
        body: JSON.stringify({ webhook_secret }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "github_connect_failed");
      }
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["service-connections"] });
    },
  });
}
