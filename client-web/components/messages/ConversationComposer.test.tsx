import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ConversationComposer } from "./ConversationComposer";

vi.mock("@/components/messages/GifSearchPanel", () => ({
  GifSearchPanel: () => <div>GIF search</div>,
}));

afterEach(cleanup);

function renderComposer(onSend = vi.fn()) {
  render(
    <ConversationComposer
      allowAttachments
      attachmentLabel="Attach files"
      attachmentRemoveLabel="Remove attachment"
      attachmentRejectedText="Rejected"
      gifLabel="Search GIFs"
      gifText="GIF"
      inputLabel="Message"
      onSend={onSend}
      pending={false}
      placeholder="Write a message"
      sendLabel="Send"
    />,
  );
  return onSend;
}

describe("ConversationComposer attachments", () => {
  it("sends an attachment with optional text and clears it after success", () => {
    const onSend = renderComposer();
    const file = new File(["runbook"], "runbook.txt", { type: "text/plain" });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;

    fireEvent.change(input, { target: { files: [file] } });
    const attachmentName = screen.getByText("runbook.txt");
    const messageInput = screen.getByPlaceholderText("Write a message");
    expect(
      attachmentName.compareDocumentPosition(messageInput) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    fireEvent.change(messageInput, {
      target: { value: "See the runbook" },
    });
    expect(screen.getByRole("button", { name: "Send" })).toHaveClass("rounded-full");
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    expect(onSend).toHaveBeenCalledWith("See the runbook", expect.any(Function), [file]);
    act(() => onSend.mock.calls[0][1]());
    expect(screen.queryByText("runbook.txt")).not.toBeInTheDocument();
  });

  it("rejects oversized files before sending", () => {
    const onSend = renderComposer();
    const oversized = new File([new Uint8Array(5 * 1024 * 1024 + 1)], "huge.txt", {
      type: "text/plain",
    });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;

    fireEvent.change(input, { target: { files: [oversized] } });

    expect(screen.getByRole("alert")).toHaveTextContent("Rejected");
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
    expect(onSend).not.toHaveBeenCalled();
  });
});
