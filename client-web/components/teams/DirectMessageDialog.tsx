"use client";

import React, { useEffect, useId, useRef, useState } from "react";
import { useTranslations } from "next-intl";
import { Send, X } from "lucide-react";
import { useAuthStore } from "@/store/auth";
import { usePrivateMessages, useSendPrivateMessage } from "@/lib/queries/privateMessages";
import { IconButton } from "@/components/ui/Button";

/**
 * A small, operational 1-to-1 conversation modal launched from the team roster.
 * Deliberately not a full chat client: one peer, message history, a composer.
 * The backend stays the authority on who may be messaged (a forbidden send
 * surfaces its `no_shared_team` error here).
 */
export function DirectMessageDialog({
  peer,
  onClose,
}: {
  peer: { user_id: string; email: string };
  onClose: () => void;
}) {
  const t = useTranslations("DirectMessages");
  const tErr = useTranslations("errors");
  const currentUserId = useAuthStore((s) => s.user?.id);
  const { data: messages, isLoading, error } = usePrivateMessages(peer.user_id);
  const send = useSendPrivateMessage();
  const [content, setContent] = useState("");
  const inputId = useId();
  const scrollRef = useRef<HTMLDivElement>(null);

  const errText = (code: string, fallback: string) => (tErr.has(code) ? tErr(code) : fallback);

  // The backend returns newest-first; display oldest-first so the newest sits
  // at the bottom next to the composer, like a chat.
  const ordered = messages ? [...messages].reverse() : [];

  // Keep the latest message in view whenever the conversation changes.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = content.trim();
    if (!text) return;
    send.mutate({ recipientId: peer.user_id, content: text }, { onSuccess: () => setContent("") });
  };

  return (
    <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
      <div className="surface flex h-[32rem] w-full max-w-md flex-col rounded-md shadow-2xl">
        <div className="border-border flex items-center justify-between gap-2 border-b px-4 py-3">
          <div className="min-w-0">
            <h2 className="text-text truncate text-sm font-semibold">{t("title")}</h2>
            <p className="text-muted/70 truncate text-xs">{peer.email}</p>
          </div>
          <IconButton onClick={onClose} label={t("close")} size="sm" variant="ghost">
            <X className="h-4 w-4" />
          </IconButton>
        </div>

        <div ref={scrollRef} className="flex-1 space-y-2 overflow-y-auto p-4">
          {isLoading ? (
            <p className="text-muted animate-pulse py-8 text-center text-xs">{t("loading")}</p>
          ) : error ? (
            <p className="text-sev-critical py-8 text-center text-xs">
              {errText(error.message, t("loadFailed"))}
            </p>
          ) : ordered.length === 0 ? (
            <p className="text-muted py-8 text-center text-xs">{t("empty")}</p>
          ) : (
            ordered.map((message) => {
              const mine = message.sender_id === currentUserId;
              return (
                <div key={message.id} className={mine ? "flex justify-end" : "flex justify-start"}>
                  <div
                    className={`max-w-[80%] rounded-md px-3 py-2 text-sm break-words whitespace-pre-wrap ${
                      mine
                        ? "bg-gold/15 text-text"
                        : "surface-subtle text-text border-border border"
                    }`}
                  >
                    {message.content}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {send.error ? (
          <p className="text-sev-critical px-4 pb-1 text-xs">
            {errText(send.error.message, t("sendFailed"))}
          </p>
        ) : null}

        <form onSubmit={handleSubmit} className="border-border flex gap-2 border-t p-3">
          <label htmlFor={inputId} className="sr-only">
            {t("placeholder")}
          </label>
          <input
            id={inputId}
            type="text"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t("placeholder")}
            autoFocus
            className="ow-input flex h-10 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
          />
          <IconButton
            type="submit"
            disabled={send.isPending || !content.trim()}
            label={t("send")}
            loading={send.isPending}
            size="lg"
            variant="primary"
          >
            <Send className="h-4 w-4" />
          </IconButton>
        </form>
      </div>
    </div>
  );
}
