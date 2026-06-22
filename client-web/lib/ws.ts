import { useEffect } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";
import { useAuthStore } from "@/store/auth";
import { useQueryClient } from "@tanstack/react-query";
import { create } from "zustand";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/ws";

/** Commands the client sends to the server (see docs/markdown/WEBSOCKET_SPEC.md). */
export type WsClientCommand =
  | { type: "auth"; token: string }
  | { type: "watch"; incident_id: string }
  | { type: "unwatch"; incident_id: string }
  | { type: "status_typing"; incident_id: string }
  | { type: "refresh_teams" };

/** Events the server pushes to the client (see docs/markdown/WEBSOCKET_SPEC.md). */
export type WsServerEvent =
  | { type: "incident_state_changed"; incident_id: string; new_state: string; by: string }
  | { type: "incident_escalated"; incident_id: string; new_severity: string; by: string }
  | { type: "incident_assigned"; incident_id: string; assigned_to: string; by: string }
  | {
      type: "timeline_entry_added";
      incident_id: string;
      entry: { entry_id: string; content: string; author: string; at: number };
    }
  | { type: "presence_update"; incident_id: string; watchers: string[] }
  | { type: "team_presence_update"; team_id: string; online_user_ids: string[] }
  | { type: "user_typing"; incident_id: string; user_id: string }
  | {
      type: "rule_triggered";
      team_id: string;
      service: string;
      rule: string;
      incident_id?: string;
    }
  | { type: "rule_failed"; team_id: string; service: string; rule: string; reason: string };

interface WsState {
  /** Presence rosters keyed by incident id. A single global roster would leak
   *  and clobber across incidents as soon as more than one war room is live
   *  (desktop, parallel panels, future multi-incident views). */
  watchersByIncident: Record<string, string[]>;
  setWatchers: (incidentId: string, watchers: string[]) => void;
  /** Transient "is typing" user ids, keyed by incident id. Each entry self-expires. */
  typingByIncident: Record<string, string[]>;
  addTypingUser: (incidentId: string, userId: string) => void;
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

/** Online member ids for a single team. */
export const useTeamOnline = (teamId: string): string[] =>
  useWsStore((s) => s.onlineByTeam[teamId] ?? EMPTY);

export function useRealtime() {
  const token = useAuthStore((s) => s.token);
  const setSendJson = useWsStore((s) => s.setSendJson);
  const queryClient = useQueryClient();

  const { sendJsonMessage, lastJsonMessage, readyState } = useWebSocket(token ? WS_URL : null, {
    shouldReconnect: () => true,
    reconnectAttempts: 10,
    reconnectInterval: 3000,
  });

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
      queryClient.invalidateQueries({ queryKey: ["timeline", incidentId] });
      sendJsonMessage({ type: "watch", incident_id: incidentId });
    }
  }, [readyState, token, sendJsonMessage, queryClient]);

  useEffect(() => {
    if (!lastJsonMessage) return;

    const event = lastJsonMessage as WsServerEvent;

    switch (event.type) {
      case "incident_state_changed":
      case "incident_escalated":
      case "incident_assigned":
        queryClient.invalidateQueries({ queryKey: ["incident", event.incident_id] });
        queryClient.invalidateQueries({ queryKey: ["incidents"] });
        break;
      case "timeline_entry_added":
        queryClient.invalidateQueries({ queryKey: ["timeline", event.incident_id] });
        break;
      case "presence_update":
        useWsStore.getState().setWatchers(event.incident_id, event.watchers || []);
        break;
      case "team_presence_update":
        useWsStore.getState().setTeamOnline(event.team_id, event.online_user_ids || []);
        break;
      case "user_typing":
        useWsStore.getState().addTypingUser(event.incident_id, event.user_id);
        break;
      case "rule_triggered":
        queryClient.invalidateQueries({ queryKey: ["incidents"] });
        break;
      case "rule_failed":
        console.error(
          `[Automation] Rule failed for ${event.service}: ${event.rule} - ${event.reason}`,
        );
        break;
    }
  }, [lastJsonMessage, queryClient]);

  return { readyState };
}
