import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiFetch } from "./api";
import { downloadConversationAttachment } from "./conversations";

vi.mock("./api", () => ({
  apiFetch: vi.fn(),
}));

const mockedApiFetch = vi.mocked(apiFetch);

const attachment = { id: "a1", file_name: "runbook.txt", media_type: "text/plain", size_bytes: 5 };

describe("downloadConversationAttachment", () => {
  let createObjectURL: ReturnType<typeof vi.fn>;
  let revokeObjectURL: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    createObjectURL = vi.fn(() => "blob:runbook");
    revokeObjectURL = vi.fn();
    Object.assign(URL, { createObjectURL, revokeObjectURL });
    mockedApiFetch.mockResolvedValue(new Response("steps", { status: 200 }));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  /**
   * Firefox ignores a synthetic click on a detached anchor, so the element must
   * be in the document at the moment `click()` fires. A Chromium-only browser
   * suite cannot observe this, which is why it is asserted here instead.
   */
  it("clicks an anchor that is connected to the document", async () => {
    let connectedAtClick: boolean | null = null;
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      connectedAtClick = this.isConnected;
    });

    await downloadConversationAttachment("/api/private-message-attachments/a1", attachment);

    expect(click).toHaveBeenCalledOnce();
    expect(connectedAtClick).toBe(true);
    click.mockRestore();
  });

  it("removes the anchor again so the document is left untouched", async () => {
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    await downloadConversationAttachment("/api/private-message-attachments/a1", attachment);

    expect(document.querySelectorAll("a[download]")).toHaveLength(0);
    click.mockRestore();
  });

  /** Revoking in the same tick as the click can cancel the transfer. */
  it("revokes the object URL only on a later task", async () => {
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    await downloadConversationAttachment("/api/private-message-attachments/a1", attachment);
    expect(revokeObjectURL).not.toHaveBeenCalled();

    await vi.runAllTimersAsync();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:runbook");
    click.mockRestore();
  });

  it("names the saved file after the attachment", async () => {
    let downloadName: string | null = null;
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloadName = this.download;
    });

    await downloadConversationAttachment("/api/private-message-attachments/a1", attachment);

    expect(downloadName).toBe("runbook.txt");
    click.mockRestore();
  });

  it("raises a translatable code when the download is refused", async () => {
    mockedApiFetch.mockResolvedValue(new Response("", { status: 403 }));

    await expect(
      downloadConversationAttachment("/api/private-message-attachments/a1", attachment),
    ).rejects.toThrow("download_attachment_failed");
    expect(createObjectURL).not.toHaveBeenCalled();
  });
});
