import { useAuthStore } from "../store/auth";
import { endSession } from "./sessionLifecycle";

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
    cache: init?.cache ?? "no-store",
    headers,
  });

  // Global 401 handling
  if (response.status === 401) {
    await endSession();

    // Redirect to login, preserving the user's locale
    if (typeof window !== "undefined") {
      const locale = getLocalePrefix();
      // A full document load, on purpose: this runs outside React, where no
      // router is reachable, and a session that just died should not leave any
      // in-memory state behind. `getLocalePrefix` returns a leading slash, so
      // the destination is absolute.
      // eslint-disable-next-line @next/next/no-location-assign-relative-destination
      window.location.href = `${locale}/login`;
    }
  }

  return response;
}
