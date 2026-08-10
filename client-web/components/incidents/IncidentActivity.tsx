"use client";

import React, { useRef, useState } from "react";
import { CircleDot, Pencil, SmilePlus } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import {
  type IncidentActivityItem,
  type TimelineReaction,
  useAddTimelineEntry,
  useAvailableReactions,
  useEditTimelineEntry,
  useIncidentActivity,
  useToggleTimelineReaction,
} from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";
import { useTypingUsers, useWsStore } from "@/lib/ws";
import { giphyEntryUrl } from "@/lib/queries/gifs";
import { Alert } from "@/components/ui/Alert";
import { Button, IconButton } from "@/components/ui/Button";
import { ConversationComposer } from "@/components/messages/ConversationComposer";
import { ReactionToggle } from "@/components/ui/ReactionToggle";
import { cn } from "@/lib/utils";
import { resolveGrouping } from "./activity-grouping";

function valueAsString(data: Record<string, unknown>, key: string) {
  const value = data[key];
  return typeof value === "string" ? value : "";
}

function SystemEventItem({
  item,
  occurrences,
}: {
  item: Extract<IncidentActivityItem, { type: "system_event" }>;
  occurrences: Extract<IncidentActivityItem, { type: "system_event" }>[];
}) {
  const t = useTranslations("Incidents");
  const locale = useLocale();
  const actor = item.actor?.email ?? t("automationActor");
  const labelValue = (value: string) => {
    const labels: Record<string, string> = {
      open: t("statusOpen"),
      acknowledged: t("statusAcknowledged"),
      escalated: t("statusEscalated"),
      resolved: t("statusResolved"),
      low: t("severityLow"),
      medium: t("severityMedium"),
      high: t("severityHigh"),
      critical: t("severityCritical"),
    };
    return labels[value] ?? value;
  };
  const from = labelValue(valueAsString(item.data, "from"));
  const to = labelValue(valueAsString(item.data, "to"));

  const description =
    item.kind === "created"
      ? t("activityCreated", { actor })
      : item.kind === "assigned"
        ? t("activityAssigned", {
            actor,
            assignee: item.subject?.email ?? t("deletedUser"),
          })
        : item.kind === "severity_changed"
          ? t("activitySeverityChanged", { actor, from, to })
          : t("activityStatusChanged", { actor, from, to });
  const timestamp = new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  });
  const hoverTimes = occurrences
    .map((occurrence) => timestamp.format(new Date(occurrence.created_at)))
    .join("\n");

  return (
    <li className="flex justify-center px-4 py-2">
      <div
        className="text-muted flex max-w-2xl flex-wrap items-center justify-center gap-x-1.5 text-center text-xs leading-5"
        title={hoverTimes}
      >
        <CircleDot className="h-3 w-3 shrink-0" aria-hidden="true" />
        <span>{description}</span>
        {occurrences.length > 1 ? (
          <span className="text-muted-2 font-mono" aria-label={`${occurrences.length}`}>
            {t("activityEventCount", { count: occurrences.length })}
          </span>
        ) : null}
        {occurrences.map((occurrence) => (
          <time key={occurrence.id} className="sr-only" dateTime={occurrence.created_at}>
            {timestamp.format(new Date(occurrence.created_at))}
          </time>
        ))}
      </div>
    </li>
  );
}

function localDayKey(value: string) {
  const date = new Date(value);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

function systemEventSignature(item: Extract<IncidentActivityItem, { type: "system_event" }>) {
  return JSON.stringify([
    item.kind,
    item.actor?.user_id ?? item.actor?.email ?? null,
    item.subject?.user_id ?? item.subject?.email ?? null,
    item.data,
  ]);
}

function NoteReactions({
  available,
  incidentId,
  entryId,
  picking,
  onPicked,
  reactions,
}: {
  available: string[];
  incidentId: string;
  entryId: string;
  picking: boolean;
  onPicked: () => void;
  reactions: TimelineReaction[];
}) {
  const toggle = useToggleTimelineReaction();

  // Persist only counted reactions; the hover menu owns the complete palette.
  const present = reactions.filter((reaction) => reaction.count > 0);
  const missing = available.filter((emoji) => !present.some((r) => r.emoji === emoji));

  if (present.length === 0 && !picking) return null;

  return (
    <div className="mt-1 flex flex-wrap items-center gap-1">
      {present.map((reaction) => (
        <ReactionToggle
          key={reaction.emoji}
          emoji={reaction.emoji}
          count={reaction.count}
          label={`${reaction.emoji} (${reaction.count})`}
          pressed={reaction.reacted}
          loading={toggle.isPending}
          onClick={() => toggle.mutate({ incidentId, entryId, emoji: reaction.emoji })}
        />
      ))}
      {picking
        ? missing.map((emoji) => (
            <ReactionToggle
              key={emoji}
              emoji={emoji}
              count={0}
              label={`${emoji} (0)`}
              pressed={false}
              loading={toggle.isPending}
              onClick={() => {
                toggle.mutate({ incidentId, entryId, emoji });
                onPicked();
              }}
            />
          ))
        : null}
    </div>
  );
}

function HumanNoteItem({
  availableReactions,
  continuesAbove,
  incidentId,
  item,
}: {
  availableReactions: string[];
  continuesAbove: boolean;
  incidentId: string;
  item: Extract<IncidentActivityItem, { type: "human_note" }>;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const currentUserId = useAuthStore((state) => state.user?.id);
  const edit = useEditTimelineEntry();
  const [editing, setEditing] = useState(false);
  const [picking, setPicking] = useState(false);
  const [draft, setDraft] = useState(item.content);
  const gifUrl = giphyEntryUrl(item.content);
  const canEdit = item.author?.user_id === currentUserId && !gifUrl;
  const mine = item.author?.user_id === currentUserId;
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const save = () => {
    const content = draft.trim();
    if (!content) return;
    edit.mutate(
      { incidentId, entryId: item.entry_id, content },
      { onSuccess: () => setEditing(false) },
    );
  };

  return (
    <li
      data-note-continues-above={continuesAbove ? "true" : undefined}
      data-note-owner={mine ? "current" : "peer"}
      className={cn(
        "group flex w-full px-4",
        mine ? "justify-end" : "justify-start",
        continuesAbove ? "mt-1" : "mt-5 first:mt-0",
      )}
    >
      <div className={cn("flex max-w-[78%] min-w-0 flex-col", mine ? "items-end" : "items-start")}>
        {!continuesAbove && !mine ? (
          <span className="text-muted mb-1 px-1 text-xs font-medium">
            {item.author?.email ?? t("deletedUser")}
          </span>
        ) : null}

        <div className="relative max-w-full">
          {editing ? null : (
            <div
              className={cn(
                "bg-panel border-border absolute -top-7 z-10 flex items-center gap-0.5 rounded-md border p-0.5 opacity-0 shadow-sm transition-opacity group-focus-within:opacity-100 group-hover:opacity-100",
                mine ? "right-0" : "left-0",
              )}
            >
              <IconButton
                label={t("addReaction")}
                size="sm"
                variant="ghost"
                onClick={() => setPicking((current) => !current)}
              >
                <SmilePlus className="h-3.5 w-3.5" aria-hidden="true" />
              </IconButton>
              {canEdit ? (
                <IconButton
                  label={t("edit")}
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    edit.reset();
                    setDraft(item.content);
                    setEditing(true);
                  }}
                >
                  <Pencil className="h-3.5 w-3.5" aria-hidden="true" />
                </IconButton>
              ) : null}
            </div>
          )}

          <div
            className={cn(
              "max-w-full border px-3 py-2 shadow-sm",
              mine
                ? "border-gold bg-gold rounded-2xl rounded-br-md"
                : "bg-panel-2 border-border rounded-2xl rounded-bl-md",
            )}
          >
            {editing ? (
              <div className="min-w-64 space-y-3">
                <label>
                  <span className="sr-only">{t("editNote")}</span>
                  <textarea
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    rows={3}
                    className="ow-input w-full rounded-md px-3 py-2 text-sm"
                  />
                </label>
                {edit.error ? (
                  <p className="text-sev-critical text-xs" role="alert">
                    {errorText(edit.error.message)}
                  </p>
                ) : null}
                <div className="flex justify-end gap-2">
                  <Button size="sm" onClick={() => setEditing(false)}>
                    {t("cancel")}
                  </Button>
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={!draft.trim()}
                    loading={edit.isPending}
                    onClick={save}
                  >
                    {t("save")}
                  </Button>
                </div>
              </div>
            ) : gifUrl ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={gifUrl}
                alt={t("gifAlt")}
                loading="lazy"
                className="max-h-72 max-w-full rounded-xl"
              />
            ) : (
              <p
                className={cn(
                  "text-sm leading-6 break-words whitespace-pre-wrap",
                  mine ? "text-bg" : "text-text",
                )}
              >
                {item.content}
              </p>
            )}
          </div>
        </div>

        <div className={cn("mt-1 flex items-center gap-2 px-1", mine && "flex-row-reverse")}>
          <time
            className="text-muted-2 text-[11px]"
            dateTime={item.created_at}
            title={new Intl.DateTimeFormat(locale, {
              dateStyle: "medium",
              timeStyle: "short",
            }).format(new Date(item.created_at))}
          >
            {new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(
              new Date(item.created_at),
            )}
            {item.edited_at ? ` · ${t("edited")}` : ""}
          </time>

          <NoteReactions
            available={availableReactions}
            incidentId={incidentId}
            entryId={item.entry_id}
            picking={picking}
            onPicked={() => setPicking(false)}
            reactions={item.reactions ?? []}
          />
        </div>
      </div>
    </li>
  );
}

function ActivityComposer({
  incidentId,
  people,
}: {
  incidentId: string;
  people: Record<string, string>;
}) {
  const t = useTranslations("Incidents");
  const addEntry = useAddTimelineEntry();
  const typingUsers = useTypingUsers(incidentId);
  const sendJson = useWsStore((state) => state.sendJson);
  const lastTypingTime = useRef(0);

  return (
    <ConversationComposer
      inputLabel={t("addNote")}
      placeholder={t("addNotePlaceholder")}
      sendLabel={t("send")}
      gifLabel={t("gifButton")}
      gifText={t("gifAlt")}
      pending={addEntry.isPending}
      onChange={() => {
        const now = Date.now();
        if (now - lastTypingTime.current > 1500) {
          sendJson({ type: "status_typing", incident_id: incidentId });
          lastTypingTime.current = now;
        }
      }}
      onSend={(content, onSuccess) => addEntry.mutate({ incidentId, content }, { onSuccess })}
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
  const locale = useLocale();
  const { data = [], error, isLoading } = useIncidentActivity(incidentId);
  const { data: availableReactions = [] } = useAvailableReactions();
  const transcriptRef = React.useRef<HTMLDivElement>(null);

  // The API is newest-first; a conversation reads oldest-first toward its composer.
  const items = React.useMemo(
    () =>
      [...data].sort(
        (left, right) => new Date(left.created_at).getTime() - new Date(right.created_at).getTime(),
      ),
    [data],
  );
  const grouping = React.useMemo(() => resolveGrouping(items), [items]);
  const rows = React.useMemo(() => {
    type SystemEvent = Extract<IncidentActivityItem, { type: "system_event" }>;
    type Row = {
      item: IncidentActivityItem;
      itemIndex: number;
      occurrences: SystemEvent[];
      showDay: boolean;
    };
    const result: Row[] = [];

    items.forEach((item, itemIndex) => {
      const day = localDayKey(item.created_at);
      const previous = result.at(-1);
      if (
        item.type === "system_event" &&
        previous?.item.type === "system_event" &&
        localDayKey(previous.item.created_at) === day &&
        systemEventSignature(previous.item) === systemEventSignature(item)
      ) {
        previous.occurrences.push(item);
        return;
      }

      result.push({
        item,
        itemIndex,
        occurrences: item.type === "system_event" ? [item] : [],
        showDay: !previous || localDayKey(previous.item.created_at) !== day,
      });
    });

    return result;
  }, [items]);

  // A room opens on what was just said, not on what was said first.
  const last = items.at(-1);
  const lastId = last ? (last.type === "human_note" ? last.entry_id : last.id) : null;
  React.useEffect(() => {
    const node = transcriptRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [lastId]);

  return (
    <section
      aria-label={t("warRoomConversation")}
      data-incident-room="true"
      className="flex min-h-0 min-w-0 flex-1 flex-col"
    >
      <div
        ref={transcriptRef}
        data-incident-transcript="true"
        className="min-h-0 flex-1 overflow-y-auto"
      >
        {isLoading ? (
          <div className="space-y-3 p-4" aria-label={t("loadingActivity")}>
            {[0, 1, 2].map((item) => (
              <div key={item} className="bg-panel-2 h-16 max-w-[70%] animate-pulse rounded-2xl" />
            ))}
          </div>
        ) : error ? (
          <div className="p-4">
            <Alert tone="danger">{t("failedToLoadActivity")}</Alert>
          </div>
        ) : data.length === 0 ? (
          <div className="flex h-full min-h-40 flex-col items-center justify-center p-8 text-center">
            <CircleDot className="text-muted mx-auto h-5 w-5" aria-hidden="true" />
            <p className="text-muted mt-3 text-sm">{t("noMessages")}</p>
          </div>
        ) : (
          <ol className="relative py-4">
            {rows.map(({ item, itemIndex, occurrences, showDay }) => (
              <React.Fragment key={item.type === "system_event" ? item.id : item.entry_id}>
                {showDay ? (
                  <li className="flex items-center gap-3 px-4 py-3" aria-hidden="true">
                    <span className="bg-border h-px flex-1" />
                    <time
                      className="text-muted-2 text-[10px] font-medium tracking-wide uppercase"
                      dateTime={item.created_at}
                    >
                      {new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(
                        new Date(item.created_at),
                      )}
                    </time>
                    <span className="bg-border h-px flex-1" />
                  </li>
                ) : null}
                {item.type === "system_event" ? (
                  <SystemEventItem item={item} occurrences={occurrences} />
                ) : (
                  <HumanNoteItem
                    availableReactions={availableReactions}
                    continuesAbove={grouping[itemIndex].continuesAbove}
                    incidentId={incidentId}
                    item={item}
                  />
                )}
              </React.Fragment>
            ))}
          </ol>
        )}
      </div>

      {canCompose ? (
        <div data-incident-composer="true" className="shrink-0">
          <ActivityComposer incidentId={incidentId} people={people} />
        </div>
      ) : null}
    </section>
  );
}
