import type { QueryClient } from "@tanstack/react-query";
import { useAuthStore } from "@/store/auth";
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

/** Purges the previous identity before making a new bearer token observable. */
export async function installSessionToken(token: string) {
  await clearIdentityState();
  useAuthStore.getState().setToken(token);
}
