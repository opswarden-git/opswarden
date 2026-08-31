"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { PanelLeftOpen, PanelRightOpen } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import {
  useMarkPrivateMessageRead,
  usePrivateMessages,
  useSendPrivateMessage,
} from "@/lib/queries/privateMessages";
import { useTeamMembers } from "@/lib/queries/teams";
import { usePrivateMessageTypingUsers, usePrivateMessageWatchers } from "@/lib/ws";
import { useAuthStore } from "@/store/auth";
import { RoomNavigation } from "@/components/messages/RoomNavigation";
import { PageContent } from "@/components/layout/PageContent";
import { PageLayout } from "@/components/layout/PageLayout";
import { RailToggle } from "@/components/layout/RailToggle";
import { Alert } from "@/components/ui/Alert";
import { IconButton } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { ConversationComposer } from "@/components/messages/ConversationComposer";
import { DirectMessageItem } from "@/components/messages/DirectMessageItem";
import { ConversationTranscript } from "@/components/messages/ConversationTranscript";
import { ConversationTranscriptSkeleton } from "@/components/messages/ConversationTranscriptSkeleton";
import { ConversationRoomSkeleton } from "@/components/messages/ConversationRoomSkeleton";
import { TeamPresenceList } from "@/components/messages/TeamPresenceList";
import { cn } from "@/lib/utils";
import { hasConversationFeature, serializeConversationAttachments } from "@/lib/conversations";
import { useConversationRoom, useConversationTyping } from "@/lib/useConversationRoom";

export function DirectMessageRoomPage({ peerId, teamId }: { peerId: string; teamId: string }) {
  const t = useTranslations("DirectMessages");
  const tTeams = useTranslations("Teams");
  const tCommon = useTranslations("Common");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const currentUserId = useAuthStore((state) => state.user?.id);
  const {
    data: members = [],
    isLoading: membersLoading,
    error: membersError,
  } = useTeamMembers(teamId);
  const peer = members.find((member) => member.user_id === peerId);
  const conversation = usePrivateMessages(
    peerId,
    !!peer && !!currentUserId && peer.user_id !== currentUserId,
  );
  const send = useSendPrivateMessage();
  const markRead = useMarkPrivateMessageRead();
  const watchers = usePrivateMessageWatchers(peerId);
  const typingUsers = usePrivateMessageTypingUsers(peerId);
  const [announcement, setAnnouncement] = useState("");
  const [encodingError, setEncodingError] = useState("");
  const [isRoomsOpen, setIsRoomsOpen] = useState(false);
  const [isRoomsRailOpen, setIsRoomsRailOpen] = useState(true);
  const [isPeopleOpen, setIsPeopleOpen] = useState(false);
  const [isPeopleRailOpen, setIsPeopleRailOpen] = useState(true);
  const hasMessageBaseline = useRef(false);
  const knownMessageIds = useRef(new Set<string>());
  const directRoom = { kind: "direct" as const, id: peerId };
  useConversationRoom(directRoom, !!peer && peer.user_id !== currentUserId);
  const signalTyping = useConversationTyping(directRoom);
  const messages = useMemo(
    () => conversation.data?.pages.flatMap((page) => page.messages) ?? [],
    [conversation.data],
  );
  const features = conversation.data?.pages[0]?.features;
  const ordered = useMemo(
    () =>
      [...messages].sort(
        (left, right) => new Date(left.created_at).getTime() - new Date(right.created_at).getTime(),
      ),
    [messages],
  );
  const errorText = (code: string, fallback: string) => (tErr.has(code) ? tErr(code) : fallback);

  const latestMessageDate = ordered[ordered.length - 1]?.created_at;
  // `mutate` is stable across renders; the mutation object around it is not, so
  // depending on the object would re-run this on every render.
  const markReadThrough = markRead.mutate;
  useEffect(() => {
    if (peerId && latestMessageDate) {
      markReadThrough({ peerId, readThrough: latestMessageDate });
    }
  }, [markReadThrough, peerId, latestMessageDate]);

  useEffect(() => {
    if (!messages.length || (!hasMessageBaseline.current && conversation.isFetching)) return;
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
  }, [conversation.isFetching, messages, peer, peerId, t]);

  if (membersLoading) {
    return (
      <PageLayout fill className="max-w-none gap-0 px-0 pt-0 pb-0 sm:px-0 md:px-0 md:pt-0 md:pb-0">
        <PageContent className="flex min-h-0 flex-1 flex-col">
          <ConversationRoomSkeleton label={t("loading")} />
        </PageContent>
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

  const roomNavigation = <RoomNavigation teamId={teamId} inDialog={isRoomsOpen} />;
  const peopleNavigation = (
    <aside
      aria-label={tTeams("members")}
      className="bg-panel/25 border-border h-full overflow-y-auto border-l"
    >
      <TeamPresenceList
        className="p-2"
        activePeerId={peer.user_id}
        members={members}
        presentUserIds={watchers}
        teamId={teamId}
      />
    </aside>
  );
  return (
    <PageLayout fill className="max-w-none gap-0 px-0 pt-0 pb-0 sm:px-0 md:px-0 md:pt-0 md:pb-0">
      <PageContent className="flex min-h-0 flex-1 flex-col">
        <div
          className={cn(
            "border-border grid min-h-0 flex-1 grid-cols-1 overflow-hidden border-y",
            isPeopleRailOpen
              ? "lg:grid-cols-[minmax(0,1fr)_19rem]"
              : "lg:grid-cols-[minmax(0,1fr)_1rem]",
            isRoomsRailOpen && !isPeopleRailOpen && "xl:grid-cols-[14rem_minmax(0,1fr)_1rem]",
            !isRoomsRailOpen && isPeopleRailOpen && "xl:grid-cols-[1rem_minmax(0,1fr)_19rem]",
            !isRoomsRailOpen && !isPeopleRailOpen && "xl:grid-cols-[1rem_minmax(0,1fr)_1rem]",
            isRoomsRailOpen && isPeopleRailOpen && "xl:grid-cols-[14rem_minmax(0,1fr)_19rem]",
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
              side="right"
              label={t(isRoomsRailOpen ? "collapseRooms" : "expandRooms")}
              onClick={() => setIsRoomsRailOpen((open) => !open)}
            />
          </div>

          <main className="relative flex min-h-0 min-w-0 flex-col">
            <h1 className="sr-only">{peer.email}</h1>
            <IconButton
              className="absolute top-2 right-11 z-20 xl:hidden"
              label={t("rooms")}
              size="sm"
              variant="ghost"
              onClick={() => setIsRoomsOpen(true)}
            >
              <PanelLeftOpen className="h-4 w-4" aria-hidden="true" />
            </IconButton>
            <IconButton
              className="absolute top-2 right-2 z-20 lg:hidden"
              label={tTeams("members")}
              size="sm"
              variant="ghost"
              onClick={() => setIsPeopleOpen(true)}
            >
              <PanelRightOpen className="h-4 w-4" aria-hidden="true" />
            </IconButton>

            <section
              aria-label={peer.email}
              className="flex min-h-0 flex-1 flex-col"
              data-direct-message-room="true"
            >
              <div className="flex min-h-0 flex-1 flex-col pb-5">
                <ConversationTranscript
                  empty={<p className="text-muted py-8 text-center text-xs">{t("empty")}</p>}
                  error={
                    <p className="text-sev-critical py-8 text-center text-xs" role="alert">
                      {errorText(conversation.error?.message ?? "", t("loadFailed"))}
                    </p>
                  }
                  getCreatedAt={(message) => message.created_at}
                  getId={(message) => message.id}
                  hasError={!!conversation.error}
                  isLoading={conversation.isLoading}
                  items={ordered}
                  loading={<ConversationTranscriptSkeleton label={t("loading")} />}
                  locale={locale}
                  loadingEarlier={conversation.isFetchingNextPage}
                  surface="direct"
                  continuesFromPrevious={(message, previous) =>
                    !!previous &&
                    previous.sender_id === message.sender_id &&
                    new Date(message.created_at).getTime() -
                      new Date(previous.created_at).getTime() <
                      5 * 60 * 1000
                  }
                  loadEarlier={
                    conversation.hasNextPage ? () => conversation.fetchNextPage() : undefined
                  }
                  loadEarlierLabel={tCommon("loadEarlier")}
                  renderItem={(message, _index, continuesAbove) => (
                    <DirectMessageItem
                      continuesAbove={continuesAbove}
                      message={message}
                      mine={message.sender_id === currentUserId}
                      peerEmail={peer.email}
                      peerId={peer.user_id}
                    />
                  )}
                />
              </div>

              <div className="shrink-0">
                <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">
                  {announcement}
                </p>
                <ConversationComposer
                  allowAttachments={hasConversationFeature(features, "attach_files")}
                  attachmentLabel={t("attachFiles")}
                  attachmentRemoveLabel={t("removeAttachment")}
                  attachmentRejectedText={t("attachmentRejected")}
                  inputLabel={t("message")}
                  placeholder={tCommon("messagePlaceholder")}
                  sendLabel={t("send")}
                  gifLabel={tCommon("gifButton")}
                  gifText={tCommon("gifAlt")}
                  pending={send.isPending}
                  error={
                    send.error || encodingError ? (
                      <p className="text-sev-critical text-xs" role="alert">
                        {encodingError || errorText(send.error?.message ?? "", t("sendFailed"))}
                      </p>
                    ) : null
                  }
                  onChange={() => {
                    signalTyping();
                  }}
                  onSend={async (content, onSuccess, files = []) => {
                    setEncodingError("");
                    try {
                      const attachments = await serializeConversationAttachments(files);
                      send.mutate(
                        { recipientId: peer.user_id, content, attachments },
                        { onSuccess },
                      );
                    } catch {
                      setEncodingError(t("attachmentReadFailed"));
                    }
                  }}
                  status={
                    typingUsers.includes(peer.user_id) ? (
                      <p className="text-muted text-xs">{t("typing", { email: peer.email })}</p>
                    ) : null
                  }
                />
              </div>
            </section>
          </main>

          <div
            className={cn(
              "relative hidden min-h-0 lg:block",
              !isPeopleRailOpen && "border-border border-l",
            )}
            data-people-rail-open={isPeopleRailOpen ? "true" : "false"}
          >
            <RailToggle
              side="left"
              label={t(isPeopleRailOpen ? "collapseMembers" : "expandMembers")}
              onClick={() => setIsPeopleRailOpen((open) => !open)}
            />
            {isPeopleRailOpen ? peopleNavigation : null}
          </div>
        </div>
      </PageContent>

      <Dialog
        open={isRoomsOpen}
        onOpenChange={setIsRoomsOpen}
        variant="sheet"
        title={t("roomTitle")}
        description={peer.email}
      >
        {roomNavigation}
      </Dialog>

      <Dialog
        open={isPeopleOpen}
        onOpenChange={setIsPeopleOpen}
        variant="sheet"
        title={tTeams("members")}
        description={peer.email}
      >
        <TeamPresenceList
          activePeerId={peer.user_id}
          members={members}
          presentUserIds={watchers}
          teamId={teamId}
        />
      </Dialog>
    </PageLayout>
  );
}
