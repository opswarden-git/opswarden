import { useEffect, useRef } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";
import { useAuthStore } from "@/store/auth";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import type { Incident } from "@/lib/queries/incidents";
import { useTranslations } from "next-intl";
import {
  createDesktopNotificationGate,
  dispatchDesktopNotification,
  type DesktopNotificationGate,
} from "@/lib/wsNotifications";

export {
  createDesktopNotificationGate,
  desktopNotificationForEvent,
  dispatchDesktopNotification,
} from "@/lib/wsNotifications";

export function webSocketUrl() {
  if (process.env.NEXT_PUBLIC_WS_URL) return process.env.NEXT_PUBLIC_WS_URL;
  if (typeof window === "undefined") return null;

  const url = new URL("/ws", window.location.origin);
  url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return url.toString();
}

/** Commands the client sends to the server (see WEBSOCKET_SPEC.md). */
export type WsClientCommand =
  | { type: "auth"; token: string }
  | { type: "watch"; incident_id: string }
  | { type: "unwatch"; incident_id: string }
  | { type: "status_typing"; incident_id: string }
  | { type: "cursor"; incident_id: string; x: number; y: number }
  | { type: "refresh_teams" };

/** Events the server pushes to the client (see WEBSOCKET_SPEC.md). */
export type WsServerEvent =
  | { type: "incident_created"; incident_id: string; severity: Incident["severity"] }
  | { type: "incident_state_changed"; incident_id: string; new_state: string; by: string }
  | { type: "incident_escalated"; incident_id: string; new_severity: string; by: string }
  | { type: "incident_assigned"; incident_id: string; assigned_to: string; by: string }
  | {
      type: "timeline_entry_added";
      incident_id: string;
      entry: { entry_id: string; content: string; author: string; at: number };
    }
  | {
      type: "timeline_entry_edited";
      incident_id: string;
      entry_id: string;
      new_content: string;
      edited_at: number;
    }
  | {
      type: "reaction_added";
      incident_id: string;
      entry_id: string;
      emoji: string;
      by: string;
    }
  | {
      type: "reaction_removed";
      incident_id: string;
      entry_id: string;
      emoji: string;
      by: string;
    }
  | {
      type: "presence_update";
      resource_id: string;
      resource_type: "incident";
      watchers: string[];
    }
  | { type: "team_presence_update"; team_id: string; online_user_ids: string[] }
  | { type: "user_typing"; incident_id: string; user_id: string }
  | {
      type: "cursor_update";
      incident_id: string;
      user_id: string;
      x: number;
      y: number;
    }
  | {
      type: "rule_triggered";
      service: string;
      rule_name: string;
      result: "incident_created" | "reaction_completed";
      incident_id: string | null;
    }
  | { type: "rule_failed"; service: string; rule_name: string; error: string }
  | { type: "member_kicked"; team_id: string; member: string; by: string }
  | {
      type: "member_banned";
      team_id: string;
      member: string;
      until: number | null;
      by: string;
    }
  | {
      type: "private_message_received";
      from: string;
      to: string;
      content: string;
      at: number;
    }
  | { type: "release_step_validated"; release_id: string; step: string; by: string }
  | { type: "release_state_changed"; release_id: string; new_state: string };

interface WsState {
  /** Presence rosters keyed by incident id. A single global roster would leak
   *  and clobber across incidents as soon as more than one war room is live
   *  (desktop, parallel panels, future multi-incident views). */
  watchersByIncident: Record<string, string[]>;
  setWatchers: (incidentId: string, watchers: string[]) => void;
  /** Transient "is typing" user ids, keyed by incident id. Each entry self-expires. */
  typingByIncident: Record<string, string[]>;
  addTypingUser: (incidentId: string, userId: string) => void;
  cursorsByIncident: Record<string, Record<string, CollaboratorCursor>>;
  setCursor: (incidentId: string, userId: string, x: number, y: number) => void;
  /** Online member ids per team (ephemeral WS presence). Pushed by the server on
   *  connect/disconnect; scoped so a client only ever holds its own teams' rosters. */
  onlineByTeam: Record<string, string[]>;
  setTeamOnline: (teamId: string, userIds: string[]) => void;
  /** Incidents this client intends to watch. Kept so a WS reopen can replay the
   *  watch commands (the server drops presence when the socket closes) and so a
   *  socket that opens after a war room mounts still establishes presence. */
  activeWatches: string[];
  watch: (incidentId: string) => void;
  unwatch: (incidentId: string) => void;
  sendJson: (msg: WsClientCommand) => void;
  setSendJson: (fn: (msg: WsClientCommand) => void) => void;
}

export interface CollaboratorCursor {
  userId: string;
  x: number;
  y: number;
  updatedAt: number;
}

const CURSOR_IDLE_MS = 1800;

export const useWsStore = create<WsState>((set, get) => ({
  watchersByIncident: {},
  setWatchers: (incidentId, watchers) =>
    set((state) => ({
      watchersByIncident: { ...state.watchersByIncident, [incidentId]: watchers },
    })),
  typingByIncident: {},
  addTypingUser: (incidentId, userId) => {
    set((state) => {
      const current = state.typingByIncident[incidentId] ?? [];
      if (current.includes(userId)) return state;
      return {
        typingByIncident: { ...state.typingByIncident, [incidentId]: [...current, userId] },
      };
    });
    setTimeout(() => {
      set((state) => {
        const current = state.typingByIncident[incidentId];
        if (!current?.includes(userId)) return state;
        return {
          typingByIncident: {
            ...state.typingByIncident,
            [incidentId]: current.filter((u) => u !== userId),
          },
        };
      });
    }, 3000);
  },
  cursorsByIncident: {},
  setCursor: (incidentId, userId, x, y) => {
    if (
      ![x, y].every(
        (coordinate) => Number.isFinite(coordinate) && coordinate >= 0 && coordinate <= 1,
      )
    ) {
      return;
    }
    const updatedAt = Date.now();
    set((state) => ({
      cursorsByIncident: {
        ...state.cursorsByIncident,
        [incidentId]: {
          ...(state.cursorsByIncident[incidentId] ?? {}),
          [userId]: { userId, x, y, updatedAt },
        },
      },
    }));
    setTimeout(() => {
      set((state) => {
        const incidentCursors = state.cursorsByIncident[incidentId];
        if (!incidentCursors || incidentCursors[userId]?.updatedAt !== updatedAt) return state;
        const { [userId]: _expired, ...remaining } = incidentCursors;
        return {
          cursorsByIncident: { ...state.cursorsByIncident, [incidentId]: remaining },
        };
      });
    }, CURSOR_IDLE_MS);
  },
  onlineByTeam: {},
  setTeamOnline: (teamId, userIds) =>
    set((state) => ({
      onlineByTeam: { ...state.onlineByTeam, [teamId]: userIds },
    })),
  activeWatches: [],
  watch: (incidentId) => {
    set((state) =>
      state.activeWatches.includes(incidentId)
        ? state
        : { activeWatches: [...state.activeWatches, incidentId] },
    );
    get().sendJson({ type: "watch", incident_id: incidentId });
  },
  unwatch: (incidentId) => {
    set((state) => ({
      activeWatches: state.activeWatches.filter((id) => id !== incidentId),
    }));
    get().sendJson({ type: "unwatch", incident_id: incidentId });
  },
  sendJson: () => {},
  setSendJson: (fn) => set({ sendJson: fn }),
}));

/** Stable empty array so per-incident selectors don't return a fresh reference
 *  each render (which would defeat zustand's referential equality check). */
const EMPTY: string[] = [];

/** Presence roster for a single incident. */
export const useWatchers = (incidentId: string): string[] =>
  useWsStore((s) => s.watchersByIncident[incidentId] ?? EMPTY);

/** Users currently typing on a single incident. */
export const useTypingUsers = (incidentId: string): string[] =>
  useWsStore((s) => s.typingByIncident[incidentId] ?? EMPTY);

const EMPTY_CURSORS: Record<string, CollaboratorCursor> = {};

/** Ephemeral collaborator pointers for a single incident room. The record is
 * selected directly so Zustand keeps a stable snapshot between WS frames. */
export const useCollaboratorCursors = (incidentId: string): Record<string, CollaboratorCursor> =>
  useWsStore((s) => s.cursorsByIncident[incidentId] ?? EMPTY_CURSORS);

/** Online member ids for a single team. */
export const useTeamOnline = (teamId: string): string[] =>
  useWsStore((s) => s.onlineByTeam[teamId] ?? EMPTY);

type ContractEvent = Extract<
  WsServerEvent,
  | { type: "presence_update" }
  | { type: "member_kicked" }
  | { type: "member_banned" }
  | { type: "private_message_received" }
  | { type: "rule_triggered" }
  | { type: "rule_failed" }
>;

/**
 * Apply contract-sensitive events outside React so their cache and store
 * effects remain directly testable alongside the Rust wire-shape tests.
 */
export function handleWsContractEvent(event: ContractEvent, queryClient: QueryClient): void {
  switch (event.type) {
    case "presence_update":
      if (event.resource_type === "incident") {
        useWsStore.getState().setWatchers(event.resource_id, event.watchers || []);
      }
      break;
    case "member_kicked":
    case "member_banned":
      // A kick or ban can change membership and clear incident assignments.
      queryClient.invalidateQueries({ queryKey: ["teams"] });
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
      queryClient.invalidateQueries({ queryKey: ["incident"] });
      if (event.member === useAuthStore.getState().user?.id) {
        // The connection's server-side membership scope is cached until asked
        // to refresh, so a removed user explicitly drops the stale team.
        useWsStore.getState().sendJson({ type: "refresh_teams" });
      } else {
        queryClient.invalidateQueries({ queryKey: ["team-members", event.team_id] });
      }
      break;
    case "private_message_received": {
      // Sender and recipient invalidate the same peer-scoped conversation;
      // no team-wide cache is touched.
      const me = useAuthStore.getState().user?.id;
      if (!me) break;
      const peer = event.from === me ? event.to : event.from;
      queryClient.invalidateQueries({ queryKey: ["private-messages", peer] });
      break;
    }
    case "rule_triggered":
      queryClient.invalidateQueries({ queryKey: ["incidents"] });
      queryClient.invalidateQueries({ queryKey: ["team-automation-runs"] });
      break;
    case "rule_failed":
      queryClient.invalidateQueries({ queryKey: ["team-automation-runs"] });
      console.error(
        `[Automation] Rule failed for ${event.service}: ${event.rule_name} - ${event.error}`,
      );
      break;
  }
}

export function useRealtime() {
  const tNotifications = useTranslations("Notifications");
  const token = useAuthStore((s) => s.token);
  const setSendJson = useWsStore((s) => s.setSendJson);
  const queryClient = useQueryClient();
  const notificationGate = useRef<DesktopNotificationGate>(createDesktopNotificationGate());

  const { sendJsonMessage, lastJsonMessage, readyState } = useWebSocket(
    token ? webSocketUrl() : null,
    {
      shouldReconnect: () => true,
      reconnectAttempts: 10,
      reconnectInterval: 3000,
    },
  );

  // Store a non-queueing sender (`keep: false`): commands sent while the socket
  // is closed are dropped, never queued. Otherwise react-use-websocket flushes a
  // pre-open `watch` *before* the OPEN effect sends `auth`, making the first
  // server frame a non-auth command — which the server closes the socket on. The
  // OPEN effect stays the single place that authenticates, then replays watches.
  useEffect(() => {
    setSendJson((msg) => sendJsonMessage(msg, false));
  }, [sendJsonMessage, setSendJson]);

  // On every (re)open: authenticate, then resync. The server replays nothing it
  // missed while we were disconnected and there is no timeline polling fallback
  // anymore, so we refetch the active REST views and re-send `watch` for every
  // incident we intend to watch (a closed socket dropped its presence server-side).
  useEffect(() => {
    if (readyState !== ReadyState.OPEN || !token) return;
    sendJsonMessage({ type: "auth", token });
    const { activeWatches } = useWsStore.getState();
    queryClient.invalidateQueries({ queryKey: ["incidents"] });
    for (const incidentId of activeWatches) {
      queryClient.invalidateQueries({ queryKey: ["incident", incidentId] });
      queryClient.invalidateQueries({ queryKey: ["activity", incidentId] });
      sendJsonMessage({ type: "watch", incident_id: incidentId });
    }
  }, [readyState, token, sendJsonMessage, queryClient]);

  useEffect(() => {
    if (!lastJsonMessage) return;

    const event = lastJsonMessage as WsServerEvent;
    if (
      event.type === "incident_created" ||
      event.type === "incident_escalated" ||
      event.type === "incident_assigned" ||
      event.type === "release_state_changed"
    ) {
      dispatchDesktopNotification(
        event,
        useAuthStore.getState().user?.id,
        (key, values) => tNotifications(key, values),
        notificationGate.current,
      );
    }

    switch (event.type) {
      case "incident_created":
        queryClient.invalidateQueries({ queryKey: ["incidents"] });
        break;
      case "incident_state_changed":
      case "incident_escalated":
      case "incident_assigned": {
        const incidentPatch: Partial<Incident> =
          event.type === "incident_state_changed"
            ? { status: event.new_state as Incident["status"] }
            : event.type === "incident_escalated"
              ? { severity: event.new_severity as Incident["severity"] }
              : { assignee: event.assigned_to };

        queryClient.setQueryData<Incident>(["incident", event.incident_id], (incident) =>
          incident ? { ...incident, ...incidentPatch } : incident,
        );
        // Queue queries carry filtered items plus global counters. The compact
        // WS event does not carry enough data to update that read model safely
        // (notably the assignee email and counter deltas), so refetch it as one
        // coherent projection instead of partially mutating cached rows.
        queryClient.invalidateQueries({ queryKey: ["incident", event.incident_id] });
        queryClient.invalidateQueries({ queryKey: ["incidents"] });
        queryClient.invalidateQueries({ queryKey: ["activity", event.incident_id] });

        break;
      }
      case "timeline_entry_added":
      case "timeline_entry_edited":
      case "reaction_added":
      case "reaction_removed":
        queryClient.invalidateQueries({ queryKey: ["activity", event.incident_id] });
        break;
      case "presence_update":
        handleWsContractEvent(event, queryClient);
        break;
      case "team_presence_update":
        useWsStore.getState().setTeamOnline(event.team_id, event.online_user_ids || []);
        // A presence change can also signal a membership change (someone just
        // joined or left this team). Refresh the roster so the member list stays
        // in sync with who is actually in the team — otherwise a joiner never
        // appears for members already viewing the roster.
        queryClient.invalidateQueries({ queryKey: ["team-members", event.team_id] });
        break;
      case "user_typing":
        useWsStore.getState().addTypingUser(event.incident_id, event.user_id);
        break;
      case "cursor_update":
        if (event.user_id !== useAuthStore.getState().user?.id) {
          useWsStore.getState().setCursor(event.incident_id, event.user_id, event.x, event.y);
        }
        break;
      case "rule_triggered":
      case "rule_failed":
        handleWsContractEvent(event, queryClient);
        break;
      case "member_kicked":
      case "member_banned": {
        handleWsContractEvent(event, queryClient);
        break;
      }
      case "private_message_received": {
        handleWsContractEvent(event, queryClient);
        break;
      }
      case "release_step_validated":
      case "release_state_changed":
        // A release changed (a step validated, or an effective-state move such as
        // an incident-driven (auto-)block). The event carries only release_id, so
        // refresh the affected release's detail and every cached release list (a
        // prefix match — the client only holds its own teams' lists anyway).
        queryClient.invalidateQueries({ queryKey: ["release", event.release_id] });
        queryClient.invalidateQueries({ queryKey: ["releases"] });
        break;
    }
  }, [lastJsonMessage, queryClient, tNotifications]);

  return { readyState };
}
