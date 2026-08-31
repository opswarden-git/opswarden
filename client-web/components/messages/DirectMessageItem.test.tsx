import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DirectMessageItem } from "./DirectMessageItem";

const { download, edit } = vi.hoisted(() => ({
  edit: { error: null, isPending: false, mutate: vi.fn(), reset: vi.fn() },
  download: vi.fn(),
}));

vi.mock("next-intl", () => ({
  useLocale: () => "en",
  useTranslations: () => (key: string) => key,
}));

vi.mock("@/lib/queries/privateMessages", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/queries/privateMessages")>();
  return {
    ...actual,
    useEditPrivateMessage: () => edit,
    downloadPrivateMessageAttachment: download,
  };
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const message = {
  id: "message-1",
  sender_id: "me",
  recipient_id: "peer",
  content: "Deployment checked",
  created_at: "2026-08-24T10:00:00Z",
  edited_at: null,
  attachments: [
    {
      id: "attachment-1",
      file_name: "runbook.pdf",
      media_type: "application/pdf",
      size_bytes: 2048,
    },
  ],
};

describe("DirectMessageItem", () => {
  it("uses the War Room interaction grammar for editing and files", async () => {
    render(
      <DirectMessageItem
        continuesAbove={false}
        message={message}
        mine
        peerEmail="peer@example.com"
        peerId="peer"
      />,
    );

    const messageActions = document.querySelector("[data-conversation-actions]");
    expect(messageActions).toHaveClass("top-1", "right-full", "opacity-0");
    expect(messageActions).not.toHaveClass("bg-panel", "border");
    expect(screen.queryByRole("button", { name: "addReaction" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "edit" }));
    const editor = screen.getByRole("textbox", { name: "editMessage" });
    expect(editor).toHaveClass("bg-transparent", "min-h-24");
    expect(editor.closest(".bg-gold")).toBeInTheDocument();
    fireEvent.change(editor, {
      target: { value: "Deployment verified" },
    });
    fireEvent.click(screen.getByRole("button", { name: "save" }));
    expect(edit.mutate).toHaveBeenCalledWith(
      { messageId: "message-1", content: "Deployment verified" },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
    act(() => edit.mutate.mock.calls[0][1].onSuccess());

    const attachment = screen.getByRole("button", { name: /runbook\.pdf/i });
    const messageText = screen.getByText("Deployment checked");
    expect(attachment.compareDocumentPosition(messageText) & Node.DOCUMENT_POSITION_FOLLOWING).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );

    fireEvent.click(attachment);
    await waitFor(() => expect(download).toHaveBeenCalledWith(message.attachments[0]));
  });
});
