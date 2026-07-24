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
    });
  });
});
