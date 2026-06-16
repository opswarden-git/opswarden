import { useEffect } from "react";
import useWebSocket, { ReadyState } from "react-use-websocket";
import { useAuthStore } from "@/store/auth";
import { useQueryClient } from "@tanstack/react-query";
import { create } from "zustand";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/ws";

interface WsState {
  watchers: string[];
  setWatchers: (watchers: string[]) => void;
  sendJson: (msg: any) => void;
  setSendJson: (fn: (msg: any) => void) => void;
}

export const useWsStore = create<WsState>((set) => ({
  watchers: [],
  setWatchers: (watchers) => set({ watchers }),
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

    const event = lastJsonMessage as any;

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
    }
  }, [lastJsonMessage, queryClient, setWatchers]);

  return { readyState };
}
