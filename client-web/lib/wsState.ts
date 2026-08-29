import { create } from "zustand";

export type WsClientCommand =
  | { type: "auth"; token: string }
  | { type: "watch"; incident_id: string }
  | { type: "unwatch"; incident_id: string }
  | { type: "status_typing"; incident_id: string }
  | { type: "cursor"; incident_id: string; x: number; y: number }
  | { type: "watch_private_message"; peer_id: string }
  | { type: "unwatch_private_message"; peer_id: string }
  | { type: "private_message_typing"; peer_id: string }
  | { type: "refresh_teams" };

export type ConversationRoom = { kind: "incident"; id: string } | { kind: "direct"; id: string };

export interface CollaboratorCursor {
  userId: string;
  x: number;
  y: number;
  updatedAt: number;
}

interface WsState {
  watchersByRoom: Record<string, string[]>;
  typingByRoom: Record<string, string[]>;
  activeRooms: ConversationRoom[];
  cursorsByIncident: Record<string, Record<string, CollaboratorCursor>>;
  onlineByTeam: Record<string, string[]>;
  setRoomWatchers: (room: ConversationRoom, users: string[]) => void;
  addRoomTypingUser: (room: ConversationRoom, userId: string) => void;
  watchRoom: (room: ConversationRoom) => void;
  unwatchRoom: (room: ConversationRoom) => void;
  signalTyping: (room: ConversationRoom) => void;
  setCursor: (incidentId: string, userId: string, x: number, y: number) => void;
  setTeamOnline: (teamId: string, userIds: string[]) => void;
  sendJson: (message: WsClientCommand) => void;
  setSendJson: (send: (message: WsClientCommand) => void) => void;
  resetSessionState: () => void;
}

const CURSOR_IDLE_MS = 1800;
const TYPING_IDLE_MS = 3000;
const EMPTY: string[] = [];
const EMPTY_CURSORS: Record<string, CollaboratorCursor> = {};
const NOOP_SEND = () => {};
let sessionGeneration = 0;

export function roomKey(room: ConversationRoom) {
  return `${room.kind}:${room.id}`;
}

function roomCommand(action: "watch" | "unwatch" | "typing", room: ConversationRoom) {
  if (room.kind === "incident") {
    if (action === "typing") return { type: "status_typing", incident_id: room.id } as const;
    return { type: action, incident_id: room.id } as const;
  }
  if (action === "typing") return { type: "private_message_typing", peer_id: room.id } as const;
  return {
    type: action === "watch" ? "watch_private_message" : "unwatch_private_message",
    peer_id: room.id,
  } as const;
}

export const useWsStore = create<WsState>((set, get) => ({
  watchersByRoom: {},
  typingByRoom: {},
  activeRooms: [],
  cursorsByIncident: {},
  onlineByTeam: {},
  setRoomWatchers: (room, users) =>
    set((state) => ({
      watchersByRoom: { ...state.watchersByRoom, [roomKey(room)]: users },
    })),
  addRoomTypingUser: (room, userId) => {
    const key = roomKey(room);
    const generation = sessionGeneration;
    set((state) => ({
      typingByRoom: {
        ...state.typingByRoom,
        [key]: Array.from(new Set([...(state.typingByRoom[key] ?? []), userId])),
      },
    }));
    setTimeout(
      () =>
        generation === sessionGeneration &&
        set((state) => ({
          typingByRoom: {
            ...state.typingByRoom,
            [key]: (state.typingByRoom[key] ?? []).filter((current) => current !== userId),
          },
        })),
      TYPING_IDLE_MS,
    );
  },
  watchRoom: (room) => {
    const key = roomKey(room);
    set((state) =>
      state.activeRooms.some((active) => roomKey(active) === key)
        ? state
        : { activeRooms: [...state.activeRooms, room] },
    );
    get().sendJson(roomCommand("watch", room));
  },
  unwatchRoom: (room) => {
    const key = roomKey(room);
    set((state) => ({
      activeRooms: state.activeRooms.filter((active) => roomKey(active) !== key),
    }));
    get().sendJson(roomCommand("unwatch", room));
  },
  signalTyping: (room) => get().sendJson(roomCommand("typing", room)),
  setCursor: (incidentId, userId, x, y) => {
    if (
      ![x, y].every(
        (coordinate) => Number.isFinite(coordinate) && coordinate >= 0 && coordinate <= 1,
      )
    )
      return;
    const updatedAt = Date.now();
    const generation = sessionGeneration;
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
      if (generation !== sessionGeneration) return;
      set((state) => {
        const cursors = state.cursorsByIncident[incidentId];
        if (!cursors || cursors[userId]?.updatedAt !== updatedAt) return state;
        const { [userId]: _expired, ...remaining } = cursors;
        return {
          cursorsByIncident: { ...state.cursorsByIncident, [incidentId]: remaining },
        };
      });
    }, CURSOR_IDLE_MS);
  },
  setTeamOnline: (teamId, userIds) =>
    set((state) => ({ onlineByTeam: { ...state.onlineByTeam, [teamId]: userIds } })),
  sendJson: NOOP_SEND,
  setSendJson: (sendJson) => set({ sendJson }),
  resetSessionState: () => {
    sessionGeneration += 1;
    set({
      watchersByRoom: {},
      typingByRoom: {},
      activeRooms: [],
      cursorsByIncident: {},
      onlineByTeam: {},
      sendJson: NOOP_SEND,
    });
  },
}));

export const useRoomWatchers = (room: ConversationRoom): string[] =>
  useWsStore((state) => state.watchersByRoom[roomKey(room)] ?? EMPTY);

export const useRoomTypingUsers = (room: ConversationRoom): string[] =>
  useWsStore((state) => state.typingByRoom[roomKey(room)] ?? EMPTY);

export const useWatchers = (incidentId: string) =>
  useRoomWatchers({ kind: "incident", id: incidentId });

export const useTypingUsers = (incidentId: string) =>
  useRoomTypingUsers({ kind: "incident", id: incidentId });

export const usePrivateMessageWatchers = (peerId: string) =>
  useRoomWatchers({ kind: "direct", id: peerId });

export const usePrivateMessageTypingUsers = (peerId: string) =>
  useRoomTypingUsers({ kind: "direct", id: peerId });

export const useCollaboratorCursors = (incidentId: string): Record<string, CollaboratorCursor> =>
  useWsStore((state) => state.cursorsByIncident[incidentId] ?? EMPTY_CURSORS);

export const useTeamOnline = (teamId: string): string[] =>
  useWsStore((state) => state.onlineByTeam[teamId] ?? EMPTY);
