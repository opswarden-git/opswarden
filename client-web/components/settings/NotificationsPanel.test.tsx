import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NotificationsPanel } from "./NotificationsPanel";

vi.mock("next-intl", () => ({
  useTranslations: () => (key: string) => key,
}));

function withPermission(
  permission: NotificationPermission | null,
  requestPermission: () => Promise<NotificationPermission> = async () => "granted",
) {
  if (permission === null) {
    // @ts-expect-error deleting the API is exactly what an old browser looks like
    delete window.Notification;
    return;
  }
  Object.defineProperty(window, "Notification", {
    configurable: true,
    writable: true,
    value: {
      permission,
      requestPermission: vi.fn(async () => {
        const answer = await requestPermission();
        (window.Notification as unknown as { permission: string }).permission = answer;
        return answer;
      }),
    },
  });
}

afterEach(() => {
  cleanup();
  window.localStorage.clear();
  vi.clearAllMocks();
});

describe("NotificationsPanel", () => {
  it("offers the button only while the answer is still open", async () => {
    withPermission("default");
    render(<NotificationsPanel />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "notificationsEnable" })).toBeInTheDocument(),
    );
    expect(screen.getAllByText("notificationsOff")).toHaveLength(2);
  });

  it("asks the browser from the click and reflects the answer", async () => {
    withPermission("default");
    render(<NotificationsPanel />);
    const button = await screen.findByRole("button", { name: "notificationsEnable" });
    fireEvent.click(button);

    await waitFor(() => expect(screen.getByText("notificationsOn")).toBeInTheDocument());
    expect(window.Notification.requestPermission).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "notificationsEnable" })).not.toBeInTheDocument();
  });

  /**
   * Firefox rejects the request when it does not follow a user gesture. The
   * panel must survive that and keep showing the real state rather than
   * pretending the answer arrived.
   */
  it("survives a rejected request", async () => {
    withPermission("default", async () => {
      throw new Error("requires user activation");
    });
    render(<NotificationsPanel />);
    fireEvent.click(await screen.findByRole("button", { name: "notificationsEnable" }));
    await waitFor(() => expect(screen.getAllByText("notificationsOff")).toHaveLength(2));
  });

  it("names the two states it cannot act on", async () => {
    withPermission("denied");
    const view = render(<NotificationsPanel />);
    await waitFor(() => expect(screen.getByText("notificationsBlocked")).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "notificationsEnable" })).not.toBeInTheDocument();

    view.unmount();
    withPermission(null);
    render(<NotificationsPanel />);
    await waitFor(() => expect(screen.getByText("notificationsUnsupported")).toBeInTheDocument());
  });

  it("persists the sound opt-in and can turn it off again", async () => {
    withPermission("granted");
    render(<NotificationsPanel />);

    fireEvent.click(screen.getByRole("button", { name: "soundsEnable" }));
    expect(window.localStorage.getItem("opswarden.notification-sounds")).toBe("true");
    expect(screen.getByRole("button", { name: "soundsDisable" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "soundsDisable" }));
    expect(window.localStorage.getItem("opswarden.notification-sounds")).toBe("false");
  });
});
