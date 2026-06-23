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

/**
 * Show a native OS notification when running in the desktop shell; otherwise do
 * nothing. Never throws: any failure (permission denied, plugin missing, IPC
 * blocked) is logged and swallowed so realtime handling is never interrupted.
 */
export async function notifyDesktop(title: string, body: string): Promise<void> {
  if (!isTauri()) return;

  try {
    const { isPermissionGranted, requestPermission, sendNotification } =
      await import("@tauri-apps/plugin-notification");

    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (!granted) return;

    sendNotification({ title, body });
  } catch (err) {
    console.warn("[desktop] notification failed:", err);
  }
}
