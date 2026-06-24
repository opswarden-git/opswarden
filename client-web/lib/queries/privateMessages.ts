import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api";

export interface PrivateMessage {
  id: string;
  sender_id: string;
  recipient_id: string;
  content: string;
  created_at: string;
}

interface ConversationResponse {
  messages: PrivateMessage[];
}

/**
 * The 1-to-1 conversation with `peerId`, newest first (as the backend returns
 * it). Keyed by the peer so the WS `private_message_received` handler can
 * invalidate exactly this conversation and nothing team-wide.
 */
export function usePrivateMessages(peerId: string) {
  return useQuery<PrivateMessage[]>({
    queryKey: ["private-messages", peerId],
    queryFn: async () => {
      const res = await apiFetch(`/api/private-messages?peer_id=${encodeURIComponent(peerId)}`);
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        throw new Error(body?.code ?? "private_messages_failed");
      }
      const body = (await res.json()) as ConversationResponse;
      return body.messages;
    },
    enabled: !!peerId,
  });
}

/** Send a private message to `recipientId`; refreshes that peer's conversation. */
export function useSendPrivateMessage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ recipientId, content }: { recipientId: string; content: string }) => {
      const res = await apiFetch("/api/private-messages", {
        method: "POST",
        body: JSON.stringify({ recipient_id: recipientId, content }),
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
