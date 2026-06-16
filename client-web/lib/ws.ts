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
  watchers: string[];
  setWatchers: (watchers: string[]) => void;
  typingUsers: string[];
  addTypingUser: (user_id: string) => void;
  sendJson: (msg: WsClientCommand) => void;
  setSendJson: (fn: (msg: WsClientCommand) => void) => void;
}

export const useWsStore = create<WsState>((set) => ({
  watchers: [],
  setWatchers: (watchers) => set({ watchers }),
  typingUsers: [],
  addTypingUser: (user_id) => {
    set((state) => ({
      typingUsers: state.typingUsers.includes(user_id)
        ? state.typingUsers
        : [...state.typingUsers, user_id],
    }));
    setTimeout(() => {
      set((state) => ({ typingUsers: state.typingUsers.filter((u) => u !== user_id) }));
    }, 3000);
  },
  sendJson: () => {},
  setSendJson: (fn) => set({ sendJson: fn }),
}));

export function useRealtime() {
  const token = useAuthStore((s) => s.token);
  const { setSendJson, setWatchers } = useWsStore();
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
        setWatchers(event.watchers || []);
        break;
      case "user_typing":
        useWsStore.getState().addTypingUser(event.user_id);
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
  }, [lastJsonMessage, queryClient, setWatchers]);

  return { readyState };
}
