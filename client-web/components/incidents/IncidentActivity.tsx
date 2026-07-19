"use client";

import React, { useRef, useState } from "react";
import { Activity, Bot, Check, CircleDot, Pencil, Send, UserRound } from "lucide-react";
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
import { ReactionToggle } from "@/components/ui/ReactionToggle";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { GifSearchPanel } from "./GifSearchPanel";

function valueAsString(data: Record<string, unknown>, key: string) {
  const value = data[key];
  return typeof value === "string" ? value : "";
}

function SystemEventItem({
  item,
}: {
  item: Extract<IncidentActivityItem, { type: "system_event" }>;
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

  return (
    <li className="relative flex gap-3 pb-5 last:pb-0">
      <div className="bg-panel border-border relative z-10 flex h-7 w-7 shrink-0 items-center justify-center rounded-full border">
        {item.actor ? (
          <UserRound className="text-muted h-3.5 w-3.5" aria-hidden="true" />
        ) : (
          <Bot className="text-muted h-3.5 w-3.5" aria-hidden="true" />
        )}
      </div>
      <div className="min-w-0 flex-1 pt-0.5">
        <p className="text-text text-sm leading-5">{description}</p>
        <time className="text-muted mt-0.5 block text-xs" dateTime={item.created_at}>
          {new Intl.DateTimeFormat(locale, {
            dateStyle: "medium",
            timeStyle: "short",
          }).format(new Date(item.created_at))}
        </time>
      </div>
    </li>
  );
}

function NoteReactions({
  available,
  incidentId,
  entryId,
  reactions,
}: {
  available: string[];
  incidentId: string;
  entryId: string;
  reactions: TimelineReaction[];
}) {
  const toggle = useToggleTimelineReaction();
  const extras = reactions
    .map((reaction) => reaction.emoji)
    .filter((emoji) => !available.includes(emoji));

  return (
    <div className="mt-3 flex flex-wrap items-center gap-1">
      {[...available, ...extras].map((emoji) => {
        const reaction = reactions.find((candidate) => candidate.emoji === emoji);
        return (
          <ReactionToggle
            key={emoji}
            emoji={emoji}
            count={reaction?.count ?? 0}
            label={`${emoji} (${reaction?.count ?? 0})`}
            pressed={reaction?.reacted ?? false}
            loading={toggle.isPending}
            onClick={() => toggle.mutate({ incidentId, entryId, emoji })}
          />
        );
      })}
    </div>
  );
}

function HumanNoteItem({
  availableReactions,
  incidentId,
  item,
}: {
  availableReactions: string[];
  incidentId: string;
  item: Extract<IncidentActivityItem, { type: "human_note" }>;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const locale = useLocale();
  const currentUserId = useAuthStore((state) => state.user?.id);
  const edit = useEditTimelineEntry();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(item.content);
  const gifUrl = giphyEntryUrl(item.content);
  const canEdit = item.author?.user_id === currentUserId && !gifUrl;
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
    <li className="surface border-border relative z-10 mb-5 rounded-md border p-4 last:mb-0 sm:p-5">
      <div className="mb-3 flex min-w-0 items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <span className="bg-panel-2 text-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold uppercase">
            {item.author?.email.slice(0, 2) ?? "?"}
          </span>
          <div className="min-w-0">
            <p className="text-text truncate text-sm font-medium">
              {item.author?.email ?? t("deletedUser")}
            </p>
            <time className="text-muted block text-xs" dateTime={item.created_at}>
              {new Intl.DateTimeFormat(locale, {
                dateStyle: "medium",
                timeStyle: "short",
              }).format(new Date(item.created_at))}
              {item.edited_at ? ` · ${t("edited")}` : ""}
            </time>
          </div>
        </div>
        {canEdit && !editing ? (
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

      {editing ? (
        <div className="space-y-3">
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            rows={3}
            className="ow-input w-full rounded-md px-3 py-2 text-sm"
          />
          {edit.error ? (
            <p className="text-sev-critical text-xs">{errorText(edit.error.message)}</p>
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
              <Check className="h-3.5 w-3.5" aria-hidden="true" />
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
          className="max-h-72 max-w-full rounded-md"
        />
      ) : (
        <p className="text-text text-sm leading-6 whitespace-pre-wrap">{item.content}</p>
      )}

      <NoteReactions
        available={availableReactions}
        incidentId={incidentId}
        entryId={item.entry_id}
        reactions={item.reactions ?? []}
      />
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
  const [content, setContent] = useState("");
  const [showGifPanel, setShowGifPanel] = useState(false);
  const typingUsers = useTypingUsers(incidentId);
  const sendJson = useWsStore((state) => state.sendJson);
  const lastTypingTime = useRef(0);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const note = content.trim();
    if (!note) return;
    addEntry.mutate({ incidentId, content: note }, { onSuccess: () => setContent("") });
  };

  return (
    <div className="surface border-border rounded-md border p-4">
      {showGifPanel ? (
        <GifSearchPanel
          disabled={addEntry.isPending}
          onClose={() => setShowGifPanel(false)}
          onSelect={(url) =>
            addEntry.mutate(
              { incidentId, content: `giphy:${url}` },
              { onSuccess: () => setShowGifPanel(false) },
            )
          }
        />
      ) : null}
      <form onSubmit={submit} className="flex items-center gap-2">
        <ToggleButton
          size="lg"
          pressed={showGifPanel}
          onClick={() => setShowGifPanel((current) => !current)}
          aria-label={t("gifButton")}
        >
          GIF
        </ToggleButton>
        <label className="min-w-0 flex-1">
          <span className="sr-only">{t("addNote")}</span>
          <input
            value={content}
            onChange={(event) => {
              setContent(event.target.value);
              const now = Date.now();
              if (now - lastTypingTime.current > 1500) {
                sendJson({ type: "status_typing", incident_id: incidentId });
                lastTypingTime.current = now;
              }
            }}
            className="ow-input h-10 w-full min-w-0 rounded-md px-3 text-sm"
            placeholder={t("addNotePlaceholder")}
          />
        </label>
        <IconButton
          type="submit"
          label={t("send")}
          size="lg"
          variant="primary"
          disabled={!content.trim()}
          loading={addEntry.isPending}
        >
          <Send className="h-4 w-4" aria-hidden="true" />
        </IconButton>
      </form>
      {typingUsers.length > 0 ? (
        <p className="text-muted mt-2 text-xs">
          {typingUsers.length === 1
            ? t("typingOne", { user: people[typingUsers[0]] ?? t("teamMember") })
            : t("typingMany", { count: typingUsers.length })}
        </p>
      ) : null}
    </div>
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
  const { data = [], error, isLoading } = useIncidentActivity(incidentId);
  const { data: availableReactions = [] } = useAvailableReactions();

  return (
    <section aria-labelledby="activity-title" className="min-w-0 space-y-4">
      <div className="flex items-center gap-2">
        <Activity className="text-muted h-4 w-4" aria-hidden="true" />
        <h2 id="activity-title" className="text-text text-base font-semibold">
          {t("activity")}
        </h2>
      </div>

      {canCompose ? <ActivityComposer incidentId={incidentId} people={people} /> : null}

      {isLoading ? (
        <div className="space-y-3" aria-label={t("loadingActivity")}>
          {[0, 1, 2].map((item) => (
            <div key={item} className="surface h-24 animate-pulse rounded-md" />
          ))}
        </div>
      ) : error ? (
        <Alert tone="danger">{t("failedToLoadActivity")}</Alert>
      ) : data.length === 0 ? (
        <div className="surface border-border rounded-md border p-8 text-center">
          <CircleDot className="text-muted mx-auto h-5 w-5" aria-hidden="true" />
          <p className="text-text mt-3 text-sm font-medium">{t("noActivity")}</p>
          <p className="text-muted mt-1 text-xs">{t("noActivityDescription")}</p>
        </div>
      ) : (
        <ol className="before:bg-border relative space-y-0 before:absolute before:top-3 before:bottom-3 before:left-3.5 before:w-px">
          {data.map((item) =>
            item.type === "system_event" ? (
              <SystemEventItem key={item.id} item={item} />
            ) : (
              <HumanNoteItem
                key={item.entry_id}
                availableReactions={availableReactions}
                incidentId={incidentId}
                item={item}
              />
            ),
          )}
        </ol>
      )}
    </section>
  );
}
