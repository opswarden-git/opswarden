// Native desktop notifications, isolated from the rest of the app.
//
// This is a no-op everywhere except inside the Tauri desktop shell
// (client-desktop). In SSR and in a normal browser it returns immediately, so
// callers (e.g. the realtime hook) can fire it unconditionally. The Tauri
// notification plugin is loaded lazily so it never ships in the web bundle's
// initial chunks and is only resolved when actually running in the desktop app.

/** True only inside a Tauri webview (not SSR, not a normal browser tab). */
function isTauri(): boolean {
  return (
    typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/** Notifications are useful only while the application is outside the user's attention. */
export function shouldShowDesktopNotification(): boolean {
  if (typeof document === "undefined") return false;
  return document.visibilityState === "hidden" || !document.hasFocus();
}

/**
 * Show a native OS notification when running in the desktop shell; otherwise do
 * nothing. Never throws: any failure (permission denied, plugin missing, IPC
 * blocked) is logged and swallowed so realtime handling is never interrupted.
 */
export async function notifyDesktop(title: string, body: string): Promise<void> {
  try {
    // Check again here as focus may have returned after the realtime event was
    // dispatched but before a permission prompt or lazy import completed.
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
