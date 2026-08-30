// Native desktop notifications, isolated from the rest of the app.
//
// This is a unified bridge for both normal web browsers and the Tauri desktop
// shell (client-desktop). Callers (e.g. settings panels and realtime hooks) can
// query permissions and trigger notifications through a single API without
// needing browser/Tauri branch checks.

export type NotificationPermissionState = "unsupported" | "default" | "granted" | "denied";

/** True only inside a Tauri webview (not SSR, not a normal browser tab). */
export function isTauri(): boolean {
  return (
    typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/** Check whether notifications are supported in the current environment. */
export function isNotificationSupported(): boolean {
  if (typeof window === "undefined") return false;
  if (isTauri()) return true;
  return "Notification" in window;
}

/** Get the current notification permission state for Web or Tauri. */
export async function getNotificationPermission(): Promise<NotificationPermissionState> {
  if (!isNotificationSupported()) return "unsupported";
  try {
    if (isTauri()) {
      const { isPermissionGranted } = await import("@tauri-apps/plugin-notification");
      const granted = await isPermissionGranted();
      return granted ? "granted" : "default";
    }
    if ("Notification" in window) {
      return window.Notification.permission as NotificationPermissionState;
    }
  } catch (err) {
    console.warn("[desktop] permission check failed:", err);
  }
  return "unsupported";
}

/** Request notification permission from the user in Web or Tauri. */
export async function requestNotificationPermission(): Promise<NotificationPermissionState> {
  if (!isNotificationSupported()) return "unsupported";
  try {
    if (isTauri()) {
      const { requestPermission } = await import("@tauri-apps/plugin-notification");
      const res = await requestPermission();
      return res === "granted" ? "granted" : "denied";
    }
    if ("Notification" in window) {
      const res = await window.Notification.requestPermission();
      return res as NotificationPermissionState;
    }
  } catch (err) {
    console.warn("[desktop] permission request failed:", err);
  }
  return "unsupported";
}

/** Notifications are useful only while the application is outside the user's attention. */
export function shouldShowDesktopNotification(): boolean {
  if (typeof document === "undefined") return false;
  return document.visibilityState === "hidden" || !document.hasFocus();
}

/**
 * Show a native OS notification when running in Desktop or Web. Never throws:
 * any failure (permission denied, plugin missing, IPC blocked) is logged and
 * swallowed so realtime handling is never interrupted.
 */
export async function notifyDesktop(title: string, body: string): Promise<void> {
  try {
    if (!shouldShowDesktopNotification()) return;

    if (isTauri()) {
      const { isPermissionGranted, requestPermission, sendNotification } =
        await import("@tauri-apps/plugin-notification");

      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      if (!granted) return;

      sendNotification({ title, body, silent: true });
      return;
    }

    if (typeof window === "undefined" || !("Notification" in window)) return;
    let permission = window.Notification.permission;
    if (permission === "default") permission = await window.Notification.requestPermission();
    if (permission === "granted") {
      new window.Notification(title, {
        body,
        icon: "/assets/icon-192.png",
        silent: true,
      });
    }
  } catch (err) {
    console.warn("[desktop] notification failed:", err);
  }
}
