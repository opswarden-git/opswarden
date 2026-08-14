import { notifyDesktop } from "@/lib/desktopNotify";
import type { WsServerEvent } from "@/lib/ws";

type NotificationEvent = Extract<
  WsServerEvent,
  | { type: "incident_created" }
  | { type: "incident_escalated" }
  | { type: "incident_assigned" }
  | { type: "release_state_changed" }
>;

type NotificationTranslator = (
  key:
    | "incidentAssignedTitle"
    | "incidentCriticalTitle"
    | "incidentEscalatedTitle"
    | "incidentReference"
    | "releaseBlockedTitle"
    | "releaseBlockedBody",
  values?: Record<string, string>,
) => string;

export type DesktopNotification = {
  body: string;
  fingerprint: string;
  title: string;
};

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

  if (event.new_state !== "blocked") return null;
  return {
    fingerprint: `release-blocked:${event.release_id}`,
    title: translate("releaseBlockedTitle"),
    body: translate("releaseBlockedBody", { id: event.release_id.slice(0, 8) }),
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
): boolean {
  const notification = desktopNotificationForEvent(event, currentUserId, translate);
  if (!notification || !shouldDeliver(event, notification.fingerprint)) return false;
  void notify(notification.title, notification.body);
  return true;
}
