import { notifyDesktop, shouldShowDesktopNotification } from "@/lib/desktopNotify";
import { playNotificationSound, type NotificationSound } from "@/lib/notificationSounds";
import type { WsServerEvent } from "@/lib/ws";

type NotificationEvent = Extract<
  WsServerEvent,
  | { type: "incident_created" }
  | { type: "incident_state_changed" }
  | { type: "incident_escalated" }
  | { type: "incident_assigned" }
  | { type: "timeline_entry_added" }
  | { type: "private_message_received" }
  | { type: "release_step_validated" }
  | { type: "release_state_changed" }
>;

type NotificationTranslator = (
  key:
    | "incidentAssignedTitle"
    | "incidentCriticalTitle"
    | "incidentEscalatedTitle"
    | "incidentReference"
    | "incidentStateTitle"
    | "incidentStateBody"
    | "warRoomMessageTitle"
    | "warRoomMessageBody"
    | "directMessageTitle"
    | "directMessageBody"
    | "releaseStepTitle"
    | "releaseStepBody"
    | "releaseStateTitle"
    | "releaseStateBody"
    | "releaseBlockedTitle"
    | "releaseBlockedBody",
  values?: Record<string, string>,
) => string;

export type DesktopNotification = {
  body: string;
  fingerprint: string;
  title: string;
};

export function notificationSoundForEvent(event: NotificationEvent): NotificationSound | null {
  if (event.type === "timeline_entry_added" || event.type === "private_message_received") {
    return "message";
  }
  if (event.type === "release_state_changed" && event.new_state === "completed") {
    return "release-completed";
  }
  return null;
}

export function desktopNotificationForEvent(
  event: NotificationEvent,
  currentUserId: string | undefined,
  translate: NotificationTranslator,
): DesktopNotification | null {
  if (event.type === "incident_created") {
    if (event.severity !== "critical") return null;
    return {
      fingerprint: `incident-created:${event.incident_id}:critical`,
      title: translate("incidentCriticalTitle"),
      body: translate("incidentReference", { id: event.incident_id.slice(0, 8) }),
    };
  }

  if (event.type === "incident_assigned") {
    if (!currentUserId || event.assigned_to !== currentUserId || event.by === currentUserId) {
      return null;
    }
    return {
      fingerprint: `incident-assigned:${event.incident_id}:${event.assigned_to}:${event.by}`,
      title: translate("incidentAssignedTitle"),
      body: translate("incidentReference", { id: event.incident_id.slice(0, 8) }),
    };
  }

  if (event.type === "incident_state_changed") {
    if (!currentUserId || event.by === currentUserId) return null;
    return {
      fingerprint: `incident-state:${event.incident_id}:${event.new_state}:${event.by}`,
      title: translate("incidentStateTitle", { state: event.new_state }),
      body: translate("incidentStateBody", {
        id: event.incident_id.slice(0, 8),
        state: event.new_state,
      }),
    };
  }

  if (event.type === "incident_escalated") {
    if (
      !currentUserId ||
      !["high", "critical"].includes(event.new_severity) ||
      event.by === currentUserId
    ) {
      return null;
    }
    return {
      fingerprint: `incident-escalated:${event.incident_id}:${event.new_severity}:${event.by}`,
      title:
        event.new_severity === "critical"
          ? translate("incidentCriticalTitle")
          : translate("incidentEscalatedTitle", { severity: event.new_severity }),
      body: translate("incidentReference", { id: event.incident_id.slice(0, 8) }),
    };
  }

  if (event.type === "timeline_entry_added") {
    if (!currentUserId || event.entry.author === currentUserId) return null;
    return {
      fingerprint: `incident-message:${event.incident_id}:${event.entry.entry_id}`,
      title: translate("warRoomMessageTitle"),
      body: translate("warRoomMessageBody", {
        id: event.incident_id.slice(0, 8),
        preview: event.entry.content.slice(0, 120),
      }),
    };
  }

  if (event.type === "private_message_received") {
    if (!currentUserId || event.to !== currentUserId || event.from === currentUserId) return null;
    return {
      fingerprint: `direct-message:${event.from}:${event.at}`,
      title: translate("directMessageTitle"),
      body: translate("directMessageBody", { preview: event.content.slice(0, 120) }),
    };
  }

  if (event.type === "release_step_validated") {
    if (!currentUserId || event.by === currentUserId) return null;
    return {
      fingerprint: `release-step:${event.release_id}:${event.step}:${event.by}`,
      title: translate("releaseStepTitle"),
      body: translate("releaseStepBody", {
        id: event.release_id.slice(0, 8),
        step: event.step,
      }),
    };
  }

  return event.new_state === "blocked"
    ? {
        fingerprint: `release-blocked:${event.release_id}`,
        title: translate("releaseBlockedTitle"),
        body: translate("releaseBlockedBody", { id: event.release_id.slice(0, 8) }),
      }
    : {
        fingerprint: `release-state:${event.release_id}:${event.new_state}`,
        title: translate("releaseStateTitle", { state: event.new_state }),
        body: translate("releaseStateBody", {
          id: event.release_id.slice(0, 8),
          state: event.new_state,
        }),
      };
}

const NOTIFICATION_DEDUP_WINDOW_MS = 30_000;

export type DesktopNotificationGate = (
  event: NotificationEvent,
  fingerprint: string,
  now?: number,
) => boolean;

export function createDesktopNotificationGate(): DesktopNotificationGate {
  const events = new WeakSet<object>();
  const fingerprints = new Map<string, number>();

  return (event, fingerprint, now = Date.now()) => {
    if (events.has(event)) return false;
    events.add(event);

    for (const [key, timestamp] of fingerprints) {
      if (now - timestamp >= NOTIFICATION_DEDUP_WINDOW_MS) fingerprints.delete(key);
    }
    if (fingerprints.has(fingerprint)) return false;
    fingerprints.set(fingerprint, now);
    return true;
  };
}

export function dispatchDesktopNotification(
  event: NotificationEvent,
  currentUserId: string | undefined,
  translate: NotificationTranslator,
  shouldDeliver: DesktopNotificationGate,
  notify: (title: string, body: string) => void | Promise<void> = notifyDesktop,
  playSound: (sound: NotificationSound) => void | Promise<unknown> = playNotificationSound,
): boolean {
  if (!shouldShowDesktopNotification()) return false;
  const notification = desktopNotificationForEvent(event, currentUserId, translate);
  if (!notification || !shouldDeliver(event, notification.fingerprint)) return false;
  void notify(notification.title, notification.body);
  const sound = notificationSoundForEvent(event);
  if (sound) void playSound(sound);
  return true;
}
