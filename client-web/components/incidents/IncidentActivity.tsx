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
import { HumanNoteItem } from "./HumanNoteItem";

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
