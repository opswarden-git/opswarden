import { useAuthStore } from "../store/auth";

/**
 * Extracts the current locale from the URL path.
 * Defaults to '/en' if it cannot be determined.
 */
function getLocalePrefix(): string {
  if (typeof window === "undefined") return "/en";
  const path = window.location.pathname;
  if (path.startsWith("/fr/") || path === "/fr") return "/fr";
  return "/en"; // Default locale
}

/**
 * A wrapper around native fetch that automatically injects the Bearer token
 * and handles global 401 Unauthorized responses.
 */
export async function apiFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const token = useAuthStore.getState().token;

  const headers = new Headers(init?.headers);
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  // We assume JSON payloads primarily
  if (!headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const response = await fetch(input, {
    ...init,
    headers,
  });

  // Global 401 handling
  if (response.status === 401) {
    // Clear the store to avoid a zombie session
    useAuthStore.getState().logout();

    // Redirect to login, preserving the user's locale
    if (typeof window !== "undefined") {
      const locale = getLocalePrefix();
      window.location.href = `${locale}/login`;
    }
  }

  return response;
}
