import { apiFetch } from "./api";

export type ConversationFeature =
  | "send_text"
  | "send_gif"
  | "edit_own_message"
  | "react"
  | "attach_files"
  | "paginated_history"
  | "presence"
  | "typing"
  | "collaborative_cursors"
  | "system_events";

export interface ConversationAttachment {
  id: string;
  file_name: string;
  media_type: string;
  size_bytes: number;
}

export interface PendingConversationAttachment {
  file_name: string;
  media_type: string;
  data_base64: string;
}

export function hasConversationFeature(
  features: readonly ConversationFeature[] | undefined,
  feature: ConversationFeature,
) {
  return features?.includes(feature) ?? false;
}

function fileAsBase64(file: File): Promise<string> {
  return file.arrayBuffer().then((buffer) => {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    for (let offset = 0; offset < bytes.length; offset += 32_768) {
      binary += String.fromCharCode(...bytes.subarray(offset, offset + 32_768));
    }
    return btoa(binary);
  });
}

export async function serializeConversationAttachments(
  files: File[],
): Promise<PendingConversationAttachment[]> {
  return Promise.all(
    files.map(async (file) => ({
      file_name: file.name,
      media_type: file.type || "application/octet-stream",
      data_base64: await fileAsBase64(file),
    })),
  );
}

/**
 * Object URLs are revoked on a later task rather than inline. Revoking in the
 * same tick as the click can cancel the transfer before the browser has read
 * the blob; a second is far past that point and costs nothing for a file the
 * server already caps at 5 MiB.
 */
const REVOKE_DELAY_MS = 1_000;

export async function downloadConversationAttachment(
  endpoint: string,
  attachment: ConversationAttachment,
) {
  const response = await apiFetch(endpoint);
  if (!response.ok) throw new Error("download_attachment_failed");
  const url = URL.createObjectURL(await response.blob());
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = attachment.file_name;
  anchor.rel = "noopener";
  // Firefox only honours a synthetic click on an anchor that is connected to
  // the document. Chromium does not, which is why a Chromium-only browser suite
  // reported this path as working.
  document.body.append(anchor);
  try {
    anchor.click();
  } finally {
    anchor.remove();
    setTimeout(() => URL.revokeObjectURL(url), REVOKE_DELAY_MS);
  }
}
