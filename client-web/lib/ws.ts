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
  | { type: "status_typing"; incident_id: string };

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
  sendJson: (msg: WsClientCommand) => void;
  setSendJson: (fn: (msg: WsClientCommand) => void) => void;
}

export const useWsStore = create<WsState>((set) => ({
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

export function useRealtime() {
  const token = useAuthStore((s) => s.token);
  const setSendJson = useWsStore((s) => s.setSendJson);
  const queryClient = useQueryClient();

  const { sendJsonMessage, lastJsonMessage, readyState } = useWebSocket(token ? WS_URL : null, {
    shouldReconnect: () => true,
    reconnectAttempts: 10,
    reconnectInterval: 3000,
  });

  useEffect(() => {
    setSendJson(sendJsonMessage);
  }, [sendJsonMessage, setSendJson]);

  useEffect(() => {
    if (readyState === ReadyState.OPEN && token) {
      sendJsonMessage({ type: "auth", token });
    }
  }, [readyState, token, sendJsonMessage]);

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
