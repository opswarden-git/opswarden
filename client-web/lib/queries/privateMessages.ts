import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";
import type {
  ConversationAttachment,
  ConversationFeature,
  PendingConversationAttachment,
} from "../conversations";
import { downloadConversationAttachment } from "../conversations";

export type PrivateMessageAttachment = ConversationAttachment;

export interface PrivateMessageReaction {
  emoji: string;
  count: number;
  reacted: boolean;
}

export interface PrivateMessage {
  id: string;
  sender_id: string;
  recipient_id: string;
  content: string;
  created_at: string;
  edited_at: string | null;
  attachments: PrivateMessageAttachment[];
  reactions: PrivateMessageReaction[];
}

export type PendingPrivateMessageAttachment = PendingConversationAttachment;

interface ConversationCursor {
  created_at: string;
  id: string;
}

interface ConversationResponse {
  messages: PrivateMessage[];
  next_cursor: ConversationCursor | null;
  features?: ConversationFeature[];
}

export function usePrivateMessages(peerId: string, enabled = true) {
  return useInfiniteQuery<ConversationResponse>({
    queryKey: ["private-messages", peerId],
    queryFn: async ({ pageParam }) => {
      const cursor = pageParam as ConversationCursor | null;
      const params = new URLSearchParams({ peer_id: peerId, limit: "50" });
      if (cursor) {
        params.set("before_created_at", cursor.created_at);
        params.set("before_id", cursor.id);
      }
      const res = await apiFetch(`/api/private-messages?${params}`);
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "private_messages_failed");
      }
      return (await res.json()) as ConversationResponse;
    },
    initialPageParam: null,
    getNextPageParam: (page) => page.next_cursor ?? undefined,
    enabled: !!peerId && enabled,
  });
}

export function useSendPrivateMessage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      recipientId,
      content,
      attachments = [],
    }: {
      recipientId: string;
      content: string;
      attachments?: PendingPrivateMessageAttachment[];
    }) => {
      const res = await apiFetch("/api/private-messages", {
        method: "POST",
        body: JSON.stringify({ recipient_id: recipientId, content, attachments }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "send_private_message_failed");
      }
      return (await res.json()) as PrivateMessage;
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["private-messages", variables.recipientId] });
    },
  });
}

export function useEditPrivateMessage(peerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ messageId, content }: { messageId: string; content: string }) => {
      const res = await apiFetch(`/api/private-messages/${messageId}`, {
        method: "PATCH",
        body: JSON.stringify({ content }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "edit_private_message_failed");
      }
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["private-messages", peerId] });
    },
  });
}

export function useTogglePrivateMessageReaction(peerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ messageId, emoji }: { messageId: string; emoji: string }) => {
      const res = await apiFetch(`/api/private-messages/${messageId}/reactions`, {
        method: "POST",
        body: JSON.stringify({ emoji }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "toggle_private_message_reaction_failed");
      }
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["private-messages", peerId] });
    },
  });
}

export async function downloadPrivateMessageAttachment(attachment: PrivateMessageAttachment) {
  await downloadConversationAttachment(
    `/api/private-message-attachments/${attachment.id}`,
    attachment,
  );
}
