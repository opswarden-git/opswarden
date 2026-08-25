"use client";

import React from "react";
import { CircleDot } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import {
  type IncidentActivityItem,
  type IncidentSeverity,
  type IncidentStatus,
  useAddTimelineEntry,
  useAvailableReactions,
  useIncidentActivity,
} from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";
import { useCollaboratorCursors, useTypingUsers, useWsStore } from "@/lib/ws";
import { Alert } from "@/components/ui/Alert";
import { ConversationComposer } from "@/components/messages/ConversationComposer";
import { ConversationTranscript } from "@/components/messages/ConversationTranscript";
import { ConversationTranscriptSkeleton } from "@/components/messages/ConversationTranscriptSkeleton";
import { hasConversationFeature, serializeConversationAttachments } from "@/lib/conversations";
import { useConversationTyping } from "@/lib/useConversationRoom";
import { groupsWithPrevious } from "./activity-grouping";
import { HumanNoteItem } from "./HumanNoteItem";
import { SeverityChip } from "./SeverityChip";
import { StateChip } from "./StateChip";
import { CollaboratorCursors } from "./CollaboratorCursors";

function valueAsString(data: Record<string, unknown>, key: string) {
  const value = data[key];
  return typeof value === "string" ? value : "";
}

function incidentStatus(value: string): IncidentStatus | null {
  return ["open", "acknowledged", "escalated", "resolved"].includes(value)
    ? (value as IncidentStatus)
    : null;
}

function incidentSeverity(value: string): IncidentSeverity | null {
  return ["low", "medium", "high", "critical"].includes(value) ? (value as IncidentSeverity) : null;
}

function SystemEventItem({
  item,
}: {
  item: Extract<IncidentActivityItem, { type: "system_event" }>;
}) {
  const t = useTranslations("Incidents");
  const locale = useLocale();
  const actor = item.actor?.email ?? t("automationActor");
  const fromValue = valueAsString(item.data, "from");
  const toValue = valueAsString(item.data, "to");
  const fromStatus = incidentStatus(fromValue);
  const toStatus = incidentStatus(toValue);
  const fromSeverity = incidentSeverity(fromValue);
  const toSeverity = incidentSeverity(toValue);
  const initialStatus = incidentStatus(valueAsString(item.data, "status"));
  const initialSeverity = incidentSeverity(valueAsString(item.data, "severity"));
  const timestamp = new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  });
  const hoverTime = timestamp.format(new Date(item.created_at));

  const transition =
    item.kind === "status_changed" && fromStatus && toStatus ? (
      <>
        <span>{t("activityStatusChanged", { actor })}</span>
        <span>{t("activityTransitionFrom")}</span>
        <StateChip status={fromStatus} />
        <span>{t("activityTransitionTo")}</span>
        <StateChip status={toStatus} />
      </>
    ) : item.kind === "severity_changed" && fromSeverity && toSeverity ? (
      <>
        <span>{t("activitySeverityChanged", { actor })}</span>
        <span>{t("activityTransitionFrom")}</span>
        <SeverityChip severity={fromSeverity} />
        <span>{t("activityTransitionTo")}</span>
        <SeverityChip severity={toSeverity} />
      </>
    ) : item.kind === "assigned" ? (
      <span>
        {t("activityAssigned", {
          actor,
          assignee: item.subject?.email ?? t("deletedUser"),
        })}
      </span>
    ) : item.kind === "created" ? (
      <>
        <span>{t("activityCreated", { actor })}</span>
        {initialStatus ? <StateChip status={initialStatus} /> : null}
        {initialSeverity ? <SeverityChip severity={initialSeverity} /> : null}
      </>
    ) : (
      <span>
        {item.kind === "status_changed"
          ? t("activityStatusChanged", { actor })
          : t("activitySeverityChanged", { actor })}
      </span>
    );

  return (
    <li data-system-event={item.kind} className="flex justify-center px-4 py-2">
      <div
        className="text-muted flex max-w-2xl flex-wrap items-center justify-center gap-x-1.5 text-center text-xs leading-5"
        title={hoverTime}
      >
        {transition}
        <time className="sr-only" dateTime={item.created_at}>
          {hoverTime}
        </time>
      </div>
    </li>
  );
}

function ActivityComposer({
  allowAttachments,
  incidentId,
  people,
}: {
  allowAttachments: boolean;
  incidentId: string;
  people: Record<string, string>;
}) {
  const t = useTranslations("Incidents");
  const tCommon = useTranslations("Common");
  const addEntry = useAddTimelineEntry();
  const [encodingError, setEncodingError] = React.useState("");
  const typingUsers = useTypingUsers(incidentId);
  const signalTyping = useConversationTyping({ kind: "incident", id: incidentId });

  return (
    <ConversationComposer
      allowAttachments={allowAttachments}
      attachmentLabel={t("attachFiles")}
      attachmentRemoveLabel={t("removeAttachment")}
      attachmentRejectedText={t("attachmentRejected")}
      inputLabel={t("addNote")}
      placeholder={tCommon("messagePlaceholder")}
      sendLabel={t("send")}
      gifLabel={tCommon("gifButton")}
      gifText={tCommon("gifAlt")}
      pending={addEntry.isPending}
      error={
        addEntry.error || encodingError ? (
          <p className="text-sev-critical text-xs" role="alert">
            {encodingError || t("actionFailed")}
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
          addEntry.mutate({ incidentId, content, attachments }, { onSuccess });
        } catch {
          setEncodingError(t("attachmentReadFailed"));
        }
      }}
      status={
        typingUsers.length > 0 ? (
          <p className="text-muted mt-2 text-xs">
            {typingUsers.length === 1
              ? t("typingOne", { user: people[typingUsers[0]] ?? t("teamMember") })
              : t("typingMany", { count: typingUsers.length })}
          </p>
        ) : null
      }
    />
  );
}

export function IncidentActivity({
  canCompose,
  incidentId,
  people,
}: {
  canCompose: boolean;
  incidentId: string;
  people: Record<string, string>;
}) {
  const t = useTranslations("Incidents");
  const tCommon = useTranslations("Common");
  const locale = useLocale();
  const {
    data = [],
    error,
    features = [],
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useIncidentActivity(incidentId);
  const { data: availableReactions = [] } = useAvailableReactions();
  const cursorMap = useCollaboratorCursors(incidentId);
  const cursors = React.useMemo(() => Object.values(cursorMap), [cursorMap]);
  const sendJson = useWsStore((state) => state.sendJson);
  const lastCursorSent = React.useRef(Number.NEGATIVE_INFINITY);

  // The API is newest-first; a conversation reads oldest-first toward its composer.
  const items = React.useMemo(
    () =>
      [...data].sort(
        (left, right) => new Date(left.created_at).getTime() - new Date(right.created_at).getTime(),
      ),
    [data],
  );
  return (
    <section
      aria-label={t("warRoomConversation")}
      data-incident-room="true"
      className="relative flex min-h-0 min-w-0 flex-1 flex-col"
      onPointerMove={(event) => {
        if (event.pointerType === "touch") return;
        const now = performance.now();
        if (now - lastCursorSent.current < 50) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (!bounds.width || !bounds.height) return;
        sendJson({
          type: "cursor",
          incident_id: incidentId,
          x: Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width)),
          y: Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height)),
        });
        lastCursorSent.current = now;
      }}
    >
      {hasConversationFeature(features, "collaborative_cursors") ? (
        <CollaboratorCursors cursors={cursors} people={people} />
      ) : null}
      <ConversationTranscript
        empty={
          <div className="flex h-full min-h-40 flex-col items-center justify-center p-8 text-center">
            <CircleDot className="text-muted mx-auto h-5 w-5" aria-hidden="true" />
            <p className="text-muted mt-3 text-sm">{t("noMessages")}</p>
          </div>
        }
        error={
          <div className="p-4">
            <Alert tone="danger">{t("failedToLoadActivity")}</Alert>
          </div>
        }
        getCreatedAt={(item) => item.created_at}
        getId={(item) => (item.type === "system_event" ? item.id : item.entry_id)}
        hasError={!!error}
        isLoading={isLoading}
        items={items}
        loading={<ConversationTranscriptSkeleton label={t("loadingActivity")} systemEvents />}
        loadEarlier={hasNextPage ? () => fetchNextPage() : undefined}
        loadEarlierLabel={tCommon("loadEarlier")}
        loadingEarlier={isFetchingNextPage}
        locale={locale}
        surface="incident"
        continuesFromPrevious={groupsWithPrevious}
        renderItem={(item, _index, continuesAbove) =>
          item.type === "system_event" ? (
            <SystemEventItem item={item} />
          ) : (
            <HumanNoteItem
              availableReactions={availableReactions}
              continuesAbove={continuesAbove}
              incidentId={incidentId}
              item={item}
            />
          )
        }
      />

      {canCompose && hasConversationFeature(features, "send_text") ? (
        <div data-incident-composer="true" className="shrink-0">
          <ActivityComposer
            allowAttachments={hasConversationFeature(features, "attach_files")}
            incidentId={incidentId}
            people={people}
          />
        </div>
      ) : null}
    </section>
  );
}
