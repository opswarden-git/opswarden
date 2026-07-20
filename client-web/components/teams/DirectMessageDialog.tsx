"use client";

import React, { useEffect, useId, useRef, useState, type ReactElement } from "react";
import { useTranslations } from "next-intl";
import { MessageSquare, Send } from "lucide-react";
import { useAuthStore } from "@/store/auth";
import { usePrivateMessages, useSendPrivateMessage } from "@/lib/queries/privateMessages";
import { IconButton } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";

/**
 * A small, operational 1-to-1 conversation modal launched from the team roster.
 * Deliberately not a full chat client: one peer, message history, a composer.
 * The backend stays the authority on who may be messaged (a forbidden send
 * surfaces its `no_shared_team` error here).
 */
export function DirectMessageDialog({
  peer,
  trigger,
}: {
  peer: { user_id: string; email: string };
  trigger: ReactElement;
}) {
  const t = useTranslations("DirectMessages");
  const tErr = useTranslations("errors");
  const currentUserId = useAuthStore((s) => s.user?.id);
  const [open, setOpen] = useState(false);
  const { data: messages, isLoading, isFetching, error } = usePrivateMessages(peer.user_id, open);
  const send = useSendPrivateMessage();
  const [content, setContent] = useState("");
  const [announcement, setAnnouncement] = useState("");
  const inputId = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const endOfThreadRef = useRef<HTMLDivElement>(null);
  const hasMessageBaseline = useRef(false);
  const knownMessageIds = useRef(new Set<string>());

  const errText = (code: string, fallback: string) => (tErr.has(code) ? tErr(code) : fallback);

  // The backend returns newest-first; display oldest-first so the newest sits
  // at the bottom next to the composer, like a chat.
  const ordered = messages ? [...messages].reverse() : [];

  // Keep the latest message in view whenever the conversation changes.
  useEffect(() => {
    endOfThreadRef.current?.scrollIntoView?.({ block: "end" });
  }, [messages]);

  // The first loaded page is history, not a notification. Afterwards, announce
  // only unseen messages sent by the peer while this exact dialog is open.
  // IDs make repeated invalidations harmless and a batch becomes one bounded
  // live-region update instead of one announcement per transport event.
  useEffect(() => {
    if (!open) return;
    if (!messages || (!hasMessageBaseline.current && isFetching)) return;

    if (!hasMessageBaseline.current) {
      knownMessageIds.current = new Set(messages.map((message) => message.id));
      hasMessageBaseline.current = true;
      return;
    }

    const received = messages.filter(
      (message) => message.sender_id === peer.user_id && !knownMessageIds.current.has(message.id),
    );
    for (const message of messages) knownMessageIds.current.add(message.id);
    if (received.length > 0) {
      setAnnouncement(t("received", { count: received.length, email: peer.email }));
    }
  }, [isFetching, messages, open, peer.email, peer.user_id, t]);

  const handleOpenChange = (nextOpen: boolean) => {
    if (nextOpen) {
      setContent("");
      setAnnouncement("");
      send.reset();
      hasMessageBaseline.current = false;
      knownMessageIds.current.clear();
    } else {
      setAnnouncement("");
    }
    setOpen(nextOpen);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = content.trim();
    if (!text) return;
    send.mutate({ recipientId: peer.user_id, content: text }, { onSuccess: () => setContent("") });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={handleOpenChange}
      trigger={trigger}
      title={t("title")}
      description={peer.email}
      closeLabel={t("close")}
      initialFocus={inputRef}
      size="sm"
      contentClassName="h-[min(32rem,calc(100dvh-2rem))]"
      bodyClassName="space-y-2 p-4"
      icon={
        <div className="bg-panel-2 text-text flex h-10 w-10 shrink-0 items-center justify-center rounded-full">
          <MessageSquare className="h-5 w-5" aria-hidden="true" />
        </div>
      }
      footer={
        <div className="w-full">
          <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">
            {announcement}
          </p>
          {send.error ? (
            <p className="text-sev-critical mb-2 text-xs" role="alert">
              {errText(send.error.message, t("sendFailed"))}
            </p>
          ) : null}
          <form onSubmit={handleSubmit} className="flex gap-2">
            <label htmlFor={inputId} className="sr-only">
              {t("placeholder")}
            </label>
            <input
              ref={inputRef}
              id={inputId}
              type="text"
              value={content}
              onChange={(event) => setContent(event.target.value)}
              placeholder={t("placeholder")}
              className="ow-input flex h-10 min-w-0 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
            />
            <IconButton
              type="submit"
              disabled={send.isPending || !content.trim()}
              label={t("send")}
              loading={send.isPending}
              size="lg"
              variant="primary"
            >
              <Send className="h-4 w-4" aria-hidden="true" />
            </IconButton>
          </form>
        </div>
      }
    >
      <div>
        {isLoading ? (
          <p className="text-muted animate-pulse py-8 text-center text-xs">{t("loading")}</p>
        ) : error ? (
          <p className="text-sev-critical py-8 text-center text-xs" role="alert">
            {errText(error.message, t("loadFailed"))}
          </p>
        ) : ordered.length === 0 ? (
          <p className="text-muted py-8 text-center text-xs">{t("empty")}</p>
        ) : (
          ordered.map((message) => {
            const mine = message.sender_id === currentUserId;
            return (
              <div
                key={message.id}
                className={mine ? "mb-2 flex justify-end" : "mb-2 flex justify-start"}
              >
                <div
                  className={`max-w-[80%] rounded-md px-3 py-2 text-sm break-words whitespace-pre-wrap ${
                    mine ? "bg-gold/15 text-text" : "surface-subtle text-text border-border border"
                  }`}
                >
                  {message.content}
                </div>
              </div>
            );
          })
        )}
        <div ref={endOfThreadRef} aria-hidden="true" />
      </div>
    </Dialog>
  );
}
