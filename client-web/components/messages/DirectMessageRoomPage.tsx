"use client";

import React, { useEffect, useRef, useState } from "react";
import { PanelLeftOpen } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { usePrivateMessages, useSendPrivateMessage } from "@/lib/queries/privateMessages";
import { giphyEntryUrl } from "@/lib/queries/gifs";
import { useTeamMembers } from "@/lib/queries/teams";
import { useAuthStore } from "@/store/auth";
import { WarRoomNavigation } from "@/components/incidents/WarRoomNavigation";
import { PageContent } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { RailToggle } from "@/components/layout/RailToggle";
import { Alert } from "@/components/ui/Alert";
import { IconButton } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { ConversationComposer } from "@/components/messages/ConversationComposer";
import { cn } from "@/lib/utils";

export function DirectMessageRoomPage({ peerId, teamId }: { peerId: string; teamId: string }) {
  const t = useTranslations("DirectMessages");
  const tIncidents = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const currentUserId = useAuthStore((state) => state.user?.id);
  const {
    data: members = [],
    isLoading: membersLoading,
    error: membersError,
  } = useTeamMembers(teamId);
  const peer = members.find((member) => member.user_id === peerId);
  const {
    data: messages,
    isLoading,
    isFetching,
    error,
  } = usePrivateMessages(peerId, !!peer && !!currentUserId && peer.user_id !== currentUserId);
  const send = useSendPrivateMessage();
  const [announcement, setAnnouncement] = useState("");
  const [isRoomsOpen, setIsRoomsOpen] = useState(false);
  const [isRoomsRailOpen, setIsRoomsRailOpen] = useState(true);
  const endOfThreadRef = useRef<HTMLDivElement>(null);
  const hasMessageBaseline = useRef(false);
  const knownMessageIds = useRef(new Set<string>());
  const ordered = messages ? [...messages].reverse() : [];

  const errorText = (code: string, fallback: string) => (tErr.has(code) ? tErr(code) : fallback);

  useEffect(() => {
    endOfThreadRef.current?.scrollIntoView?.({ block: "end" });
  }, [messages]);

  useEffect(() => {
    if (!messages || (!hasMessageBaseline.current && isFetching)) return;
    if (!hasMessageBaseline.current) {
      knownMessageIds.current = new Set(messages.map((message) => message.id));
      hasMessageBaseline.current = true;
      return;
    }

    const received = messages.filter(
      (message) => message.sender_id === peerId && !knownMessageIds.current.has(message.id),
    );
    for (const message of messages) knownMessageIds.current.add(message.id);
    if (received.length > 0 && peer) {
      setAnnouncement(t("received", { count: received.length, email: peer.email }));
    }
  }, [isFetching, messages, peer, peerId, t]);

  if (membersLoading) {
    return (
      <PageLayout fill>
        <PageContent state="loading" />
      </PageLayout>
    );
  }

  if (membersError || !peer || peer.user_id === currentUserId) {
    return (
      <PageLayout fill>
        <PageContent state="error" errorFallback={<Alert tone="danger">{t("loadFailed")}</Alert>} />
      </PageLayout>
    );
  }

  const roomNavigation = (
    <WarRoomNavigation
      activePeerId={peer.user_id}
      members={members}
      teamId={teamId}
      inDialog={isRoomsOpen}
    />
  );

  return (
    <PageLayout fill className="max-w-none gap-0 px-0 pt-0 pb-0 sm:px-0 md:px-0 md:pt-0 md:pb-0">
      <PageContent className="flex min-h-0 flex-1 flex-col">
        <div
          className={cn(
            "border-border grid min-h-0 flex-1 grid-cols-1 overflow-hidden border-y",
            isRoomsRailOpen
              ? "xl:grid-cols-[14rem_minmax(0,1fr)]"
              : "xl:grid-cols-[1rem_minmax(0,1fr)]",
          )}
        >
          <div
            className={cn(
              "relative hidden min-h-0 xl:block",
              !isRoomsRailOpen && "border-border border-r",
            )}
            data-rooms-rail-open={isRoomsRailOpen ? "true" : "false"}
          >
            {isRoomsRailOpen ? roomNavigation : null}
            <RailToggle
              className="top-1/2 right-0 -translate-y-1/2"
              direction={isRoomsRailOpen ? "left" : "right"}
              label={tIncidents(isRoomsRailOpen ? "collapseRooms" : "expandRooms")}
              onClick={() => setIsRoomsRailOpen((open) => !open)}
            />
          </div>

          <main className="relative flex min-h-0 min-w-0 flex-col">
            <h1 className="sr-only">{peer.email}</h1>

            <IconButton
              className="absolute top-2 right-2 z-20 xl:hidden"
              label={tIncidents("rooms")}
              size="sm"
              variant="ghost"
              onClick={() => setIsRoomsOpen(true)}
            >
              <PanelLeftOpen className="h-4 w-4" aria-hidden="true" />
            </IconButton>

            <section
              aria-label={peer.email}
              className="flex min-h-0 flex-1 flex-col"
              data-direct-message-room="true"
            >
              <div
                className="min-h-0 flex-1 overflow-y-auto px-4 pt-12 pb-5 sm:px-6 xl:pt-5"
                data-direct-message-transcript="true"
              >
                {isLoading ? (
                  <p className="text-muted animate-pulse py-8 text-center text-xs">
                    {t("loading")}
                  </p>
                ) : error ? (
                  <p className="text-sev-critical py-8 text-center text-xs" role="alert">
                    {errorText(error.message, t("loadFailed"))}
                  </p>
                ) : ordered.length === 0 ? (
                  <p className="text-muted py-8 text-center text-xs">{t("empty")}</p>
                ) : (
                  <div className="space-y-3">
                    {ordered.map((message) => {
                      const mine = message.sender_id === currentUserId;
                      const gifUrl = giphyEntryUrl(message.content);
                      return (
                        <article
                          key={message.id}
                          className={mine ? "flex justify-end" : "flex justify-start"}
                        >
                          <div className="max-w-[80%] sm:max-w-[70%]">
                            {!mine ? (
                              <div className="text-muted mb-1 px-1 text-[11px]">{peer.email}</div>
                            ) : null}
                            <div
                              className={
                                mine
                                  ? "bg-gold text-bg rounded-2xl rounded-br-sm px-3 py-2 text-sm break-words whitespace-pre-wrap"
                                  : "border-border bg-panel/60 text-text rounded-2xl rounded-bl-sm border px-3 py-2 text-sm break-words whitespace-pre-wrap"
                              }
                            >
                              {gifUrl ? (
                                // eslint-disable-next-line @next/next/no-img-element
                                <img
                                  src={gifUrl}
                                  alt={tIncidents("gifAlt")}
                                  loading="lazy"
                                  className="max-h-72 max-w-full rounded-xl"
                                />
                              ) : (
                                message.content
                              )}
                            </div>
                            <time
                              dateTime={message.created_at}
                              className={
                                mine
                                  ? "text-muted mt-1 block px-1 text-right text-[10px]"
                                  : "text-muted mt-1 block px-1 text-[10px]"
                              }
                            >
                              {new Intl.DateTimeFormat(locale, {
                                hour: "2-digit",
                                minute: "2-digit",
                              }).format(new Date(message.created_at))}
                            </time>
                          </div>
                        </article>
                      );
                    })}
                  </div>
                )}
                <div ref={endOfThreadRef} aria-hidden="true" />
              </div>

              <div className="shrink-0">
                <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">
                  {announcement}
                </p>
                <ConversationComposer
                  inputLabel={t("message")}
                  placeholder={t("placeholder")}
                  sendLabel={t("send")}
                  gifLabel={tIncidents("gifButton")}
                  gifText={tIncidents("gifAlt")}
                  pending={send.isPending}
                  error={
                    send.error ? (
                      <p className="text-sev-critical text-xs" role="alert">
                        {errorText(send.error.message, t("sendFailed"))}
                      </p>
                    ) : null
                  }
                  onSend={(content, onSuccess) =>
                    send.mutate({ recipientId: peer.user_id, content }, { onSuccess })
                  }
                />
              </div>
            </section>
          </main>
        </div>
      </PageContent>

      <Dialog
        open={isRoomsOpen}
        onOpenChange={setIsRoomsOpen}
        variant="sheet"
        title={tIncidents("warRoom")}
        description={peer.email}
      >
        {roomNavigation}
      </Dialog>
    </PageLayout>
  );
}
