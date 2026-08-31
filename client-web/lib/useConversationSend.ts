import React from "react";
import {
  serializeConversationAttachments,
  type PendingConversationAttachment,
} from "@/lib/conversations";

export function useConversationSend({
  onSend,
  signalTyping,
  tCommonError,
}: {
  onSend: (
    content: string,
    attachments?: PendingConversationAttachment[],
  ) => Promise<unknown> | void;
  signalTyping?: () => void;
  tCommonError?: string;
}) {
  const [encodingError, setEncodingError] = React.useState("");
  const [isSending, setIsSending] = React.useState(false);

  const send = React.useCallback(
    async (content: string, files: File[] = []) => {
      setEncodingError("");
      try {
        const serialized = await serializeConversationAttachments(files);
        setIsSending(true);
        await onSend(content, serialized.length > 0 ? serialized : undefined);
      } catch {
        setEncodingError(tCommonError ?? "Failed to send message");
      } finally {
        setIsSending(false);
      }
    },
    [onSend, tCommonError],
  );

  return {
    encodingError,
    setEncodingError,
    isSending,
    send,
    signalTyping,
  };
}
