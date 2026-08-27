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
  vi.clearAllMocks();
});

describe("NotificationsPanel", () => {
  it("offers the button only while the answer is still open", async () => {
    withPermission("default");
    render(<NotificationsPanel />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "notificationsEnable" })).toBeInTheDocument(),
    );
    expect(screen.getByText("notificationsOff")).toBeInTheDocument();
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
    await waitFor(() => expect(screen.getByText("notificationsOff")).toBeInTheDocument());
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
});
