import type { QueryClient } from "@tanstack/react-query";
import { useAuthStore, type User } from "@/store/auth";
import { useWsStore } from "./wsState";

let activeQueryClient: QueryClient | null = null;
let replaceActiveQueryClient: (() => QueryClient) | null = null;
let activeRegistration: symbol | null = null;

/** Registers the client whose private server state belongs to the current identity. */
export function registerSessionQueryClient(
  queryClient: QueryClient,
  replaceQueryClient?: () => QueryClient,
) {
  const registration = Symbol("session-query-client");
  activeQueryClient = queryClient;
  replaceActiveQueryClient = replaceQueryClient ?? null;
  activeRegistration = registration;

  return () => {
    if (activeRegistration !== registration) return;
    activeQueryClient = null;
    replaceActiveQueryClient = null;
    activeRegistration = null;
  };
}

async function clearIdentityState() {
  const queryClient = activeQueryClient;
  const cancellation = queryClient?.cancelQueries();

  // Clear synchronously first: no account-A value may be rendered while
  // cancellation settles or while account B is being installed.
  queryClient?.clear();
  if (replaceActiveQueryClient) activeQueryClient = replaceActiveQueryClient();
  useWsStore.getState().resetSessionState();
  useAuthStore.getState().logout();

  await cancellation?.catch(() => undefined);

  // A query that completed concurrently with cancellation must not repopulate
  // the cache after the first clear.
  queryClient?.clear();
}

/** Ends the current identity and purges every identity-scoped client store. */
export async function endSession() {
  await clearIdentityState();
}

/** Validates a candidate token before atomically exposing its identity. */
export async function establishSession(token: string) {
  const response = await fetch("/api/me", {
    cache: "no-store",
    headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
  });
  if (!response.ok) throw new Error("profile_load_failed");

  const user = (await response.json()) as User;
  await clearIdentityState();
  useAuthStore.getState().setSession(token, user);
  return user;
}
