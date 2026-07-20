import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PrivateMessage } from "@/lib/queries/privateMessages";
import { Button } from "@/components/ui/Button";
import { DirectMessageDialog } from "./DirectMessageDialog";

const mocks = vi.hoisted(() => ({
  query: vi.fn(),
  send: vi.fn(),
}));

vi.mock("@/lib/queries/privateMessages", () => ({
  usePrivateMessages: mocks.query,
  useSendPrivateMessage: mocks.send,
}));

vi.mock("@/store/auth", () => ({
  useAuthStore: (selector: (state: { user: { id: string } }) => unknown) =>
    selector({ user: { id: "me" } }),
}));

vi.mock("next-intl", () => ({
  useTranslations: (namespace: string) => {
    const messages: Record<string, string> = {
      title: "Direct message",
      message: "Message",
      send: "Send",
      close: "Close direct message",
      placeholder: "Write a message…",
      empty: "No messages yet. Say hello.",
      loading: "Loading conversation…",
      loadFailed: "Failed to load the conversation",
      sendFailed: "Failed to send the message",
    };
    const translate = (key: string, values?: Record<string, unknown>) => {
      if (namespace === "errors") return key;
      if (key === "received") {
        const count = Number(values?.count);
        return count === 1
          ? `New message from ${values?.email}.`
          : `${count} new messages from ${values?.email}.`;
      }
      return messages[key] ?? key;
    };
    translate.has = () => false;
    return translate;
  },
}));

const peer = { user_id: "peer", email: "peer@opswarden.local" };

function message(id: string, senderId: string, content: string): PrivateMessage {
  return {
    id,
    sender_id: senderId,
    recipient_id: senderId === "me" ? "peer" : "me",
    content,
    created_at: "2026-07-20T00:00:00Z",
  };
}

function renderDialog() {
  return render(<DirectMessageDialog peer={peer} trigger={<Button>Message peer</Button>} />);
}

beforeEach(() => {
  mocks.query.mockReturnValue({ data: [], isLoading: false, isFetching: false, error: null });
  mocks.send.mockReturnValue({
    mutate: vi.fn(),
    reset: vi.fn(),
    isPending: false,
    error: null,
  });
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("DirectMessageDialog", () => {
  it("loads only while open and restores focus after Escape", async () => {
    renderDialog();
    expect(mocks.query).toHaveBeenLastCalledWith("peer", false);

    const trigger = screen.getByRole("button", { name: "Message peer" });
    fireEvent.click(trigger);

    expect(screen.getByRole("dialog", { name: "Direct message" })).toHaveAccessibleDescription(
      "peer@opswarden.local",
    );
    expect(mocks.query).toHaveBeenLastCalledWith("peer", true);
    await waitFor(() => expect(screen.getByPlaceholderText("Write a message…")).toHaveFocus());

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("keeps history silent and coalesces unseen peer messages", async () => {
    let messages = [message("history", "peer", "Earlier")];
    mocks.query.mockImplementation(() => ({
      data: messages,
      isLoading: false,
      isFetching: false,
      error: null,
    }));
    const view = renderDialog();
    fireEvent.click(screen.getByRole("button", { name: "Message peer" }));
    await act(async () => undefined);
    expect(screen.getByRole("status")).toHaveTextContent("");

    messages = [message("new-2", "peer", "Second"), message("new-1", "peer", "First"), ...messages];
    view.rerender(<DirectMessageDialog peer={peer} trigger={<Button>Message peer</Button>} />);

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "2 new messages from peer@opswarden.local.",
      ),
    );
    view.rerender(<DirectMessageDialog peer={peer} trigger={<Button>Message peer</Button>} />);
    expect(screen.getByRole("status")).toHaveTextContent(
      "2 new messages from peer@opswarden.local.",
    );
  });

  it("announces a send error and remains dismissible during a mutation", async () => {
    const trigger = <Button>Message peer</Button>;
    mocks.send.mockReturnValue({
      mutate: vi.fn(),
      reset: vi.fn(),
      isPending: true,
      error: new Error("send_private_message_failed"),
    });
    render(<DirectMessageDialog peer={peer} trigger={trigger} />);
    fireEvent.click(screen.getByRole("button", { name: "Message peer" }));

    expect(screen.getByRole("alert")).toHaveTextContent("Failed to send the message");
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(screen.getByRole("button", { name: "Message peer" })).toHaveFocus());
  });

  it("treats the first refetch after reopening as silent history", async () => {
    let state = {
      data: [message("history", "peer", "Earlier")],
      isLoading: false,
      isFetching: false,
      error: null,
    };
    mocks.query.mockImplementation(() => state);
    const view = renderDialog();
    const trigger = screen.getByRole("button", { name: "Message peer" });
    fireEvent.click(trigger);
    await act(async () => undefined);
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    state = { ...state, isFetching: true };
    view.rerender(<DirectMessageDialog peer={peer} trigger={<Button>Message peer</Button>} />);
    fireEvent.click(screen.getByRole("button", { name: "Message peer" }));
    state = {
      ...state,
      data: [message("while-closed", "peer", "Arrived while closed"), ...state.data],
      isFetching: false,
    };
    view.rerender(<DirectMessageDialog peer={peer} trigger={<Button>Message peer</Button>} />);

    await waitFor(() => expect(screen.getByText("Arrived while closed")).toBeVisible());
    expect(screen.getByRole("status")).toHaveTextContent("");
  });
});
