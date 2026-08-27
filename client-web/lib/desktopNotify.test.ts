import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { notifyDesktop } from "./desktopNotify";

const notificationPlugin = vi.hoisted(() => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-notification", () => notificationPlugin);

beforeEach(() => {
  vi.clearAllMocks();
  notificationPlugin.isPermissionGranted.mockResolvedValue(true);
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    value: "hidden",
  });
});

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    value: "visible",
  });
});

describe("desktop notification bridge", () => {
  it("uses the native Tauri plugin while the main window is hidden", async () => {
    await notifyDesktop("Critical incident", "Incident #abcd1234");

    expect(document.visibilityState).toBe("hidden");
    expect(notificationPlugin.sendNotification).toHaveBeenCalledWith({
      title: "Critical incident",
      body: "Incident #abcd1234",
      silent: true,
    });
  });

  it("uses the browser Notification API outside Tauri", async () => {
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
    const shown = vi.fn();
    class BrowserNotification {
      static permission: NotificationPermission = "granted";
      static requestPermission = vi.fn(async () => "granted" as NotificationPermission);
      constructor(title: string, options?: NotificationOptions) {
        shown(title, options);
      }
    }
    Object.defineProperty(window, "Notification", {
      configurable: true,
      value: BrowserNotification,
    });

    await notifyDesktop("New message", "Investigating");

    expect(shown).toHaveBeenCalledWith("New message", {
      body: "Investigating",
      icon: "/assets/icon-192.png",
      silent: true,
    });
    Reflect.deleteProperty(window, "Notification");
  });

  it("stays silent while the application has the user's attention", async () => {
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    const focus = vi.spyOn(document, "hasFocus").mockReturnValue(true);

    await notifyDesktop("New message", "Already visible");

    expect(notificationPlugin.sendNotification).not.toHaveBeenCalled();
    focus.mockRestore();
  });
});
