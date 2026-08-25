"use client";

import { useCallback, useEffect, useRef } from "react";
import { type ConversationRoom, useWsStore } from "@/lib/wsState";

export function useConversationRoom(room: ConversationRoom, enabled = true) {
  const watchRoom = useWsStore((state) => state.watchRoom);
  const unwatchRoom = useWsStore((state) => state.unwatchRoom);
  const { id, kind } = room;

  useEffect(() => {
    if (!enabled) return;
    const current = { kind, id } as ConversationRoom;
    watchRoom(current);
    return () => unwatchRoom(current);
  }, [enabled, id, kind, unwatchRoom, watchRoom]);
}

export function useConversationTyping(room: ConversationRoom) {
  const signalTyping = useWsStore((state) => state.signalTyping);
  const lastSignal = useRef(0);
  const { id, kind } = room;

  return useCallback(() => {
    const now = Date.now();
    if (now - lastSignal.current <= 1500) return;
    signalTyping({ kind, id } as ConversationRoom);
    lastSignal.current = now;
  }, [id, kind, signalTyping]);
}
