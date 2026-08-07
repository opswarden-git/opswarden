"use client";

import React, { useRef, useState } from "react";
import { Activity, Bot, Check, CircleDot, Pencil, Send, SmilePlus, UserRound } from "lucide-react";
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
import { cn } from "@/lib/utils";
import { GifSearchPanel } from "./GifSearchPanel";
import { resolveGrouping } from "./activity-grouping";

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

  /*
   * Only reactions somebody actually left. The palette used to be permanent:
   * six buttons under every message, so eight entries carried forty-eight
   * controls that said nothing about the incident. Mattermost keeps a single
   * "Add Reaction" in the hover menu (`post_reaction.tsx`) and lets the counted
   * pills be the only thing that persists, because they are the only thing
   * that carries information.
   */
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
    /*
     * A flat row with a hanging avatar, not a bordered card. Three stacked
     * boxes with their own padding read as a form; a conversation is a column
     * of speech against one gutter. It is also what makes the consecutive
     * grouping visible at all — a run of messages can only look like one turn
     * if nothing draws a box around each of them.
     */
    <li
      data-note-continues-above={continuesAbove ? "true" : undefined}
      className={cn(
        "group hover:bg-panel/40 relative flex gap-3 px-3 transition-colors",
        continuesAbove ? "py-0.5" : "mt-4 pt-2 pb-0.5 first:mt-0",
      )}
    >
      <div className="w-8 shrink-0">
        {continuesAbove ? (
          <time
            className="text-muted-2 mt-1 block text-right text-xs opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100"
            dateTime={item.created_at}
            title={new Intl.DateTimeFormat(locale, {
              dateStyle: "medium",
              timeStyle: "short",
            }).format(new Date(item.created_at))}
          >
            {new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(
              new Date(item.created_at),
            )}
          </time>
        ) : (
          <span className="bg-panel-2 text-muted flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold uppercase">
            {item.author?.email.slice(0, 2) ?? "?"}
          </span>
        )}
      </div>

      {/*
       * Out of the flow, like Mattermost's post menu. Inside the header row it
       * reserved a line on every continuation message, so a run of three notes
       * came apart into three blocks separated by empty space — the opposite of
       * what the grouping is for.
       */}
      {editing ? null : (
        <div className="bg-panel border-border absolute top-0 right-2 flex items-center gap-0.5 rounded-md border p-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
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

      <div className="min-w-0 flex-1">
        {continuesAbove ? null : (
          <p className="mb-0.5 min-w-0 truncate">
            <span className="text-text text-sm font-semibold">
              {item.author?.email ?? t("deletedUser")}
            </span>
            <time className="text-muted-2 ml-2 text-xs" dateTime={item.created_at}>
              {new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(
                new Date(item.created_at),
              )}
              {item.edited_at ? ` · ${t("edited")}` : ""}
            </time>
          </p>
        )}

        {editing ? (
          <div className="space-y-3">
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
          picking={picking}
          onPicked={() => setPicking(false)}
          reactions={item.reactions ?? []}
        />
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
          {t("gifAlt")}
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
  const transcriptRef = React.useRef<HTMLDivElement>(null);

  /*
   * Oldest first, newest at the bottom, next to the composer.
   *
   * The API answers newest-first, which is a feed convention: you wrote at the
   * bottom and your message appeared at the top. Reversing it here is a
   * presentation decision and leaves the contract alone.
   */
  const items = React.useMemo(
    () =>
      [...data].sort(
        (left, right) => new Date(left.created_at).getTime() - new Date(right.created_at).getTime(),
      ),
    [data],
  );
  const grouping = React.useMemo(() => resolveGrouping(items), [items]);

  // A room opens on what was just said, not on what was said first.
  const last = items.at(-1);
  const lastId = last ? (last.type === "human_note" ? last.entry_id : last.id) : null;
  React.useEffect(() => {
    const node = transcriptRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [lastId]);

  return (
    <section
      aria-labelledby="activity-title"
      data-incident-room="true"
      className="flex min-h-0 min-w-0 flex-1 flex-col gap-4"
    >
      <div className="flex shrink-0 items-center gap-2">
        <Activity className="text-muted h-4 w-4" aria-hidden="true" />
        <h2 id="activity-title" className="text-text text-base font-semibold">
          {t("activity")}
        </h2>
      </div>

      {/*
       * Only the transcript scrolls. The heading above and the composer below
       * stay put, which is what separates a room from a record: you can read
       * back through an incident without losing the way to answer it.
       */}
      <div
        ref={transcriptRef}
        data-incident-transcript="true"
        className="min-h-0 flex-1 space-y-4 overflow-y-auto"
      >
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
          <ol className="relative">
            {items.map((item, index) =>
              item.type === "system_event" ? (
                <SystemEventItem key={item.id} item={item} />
              ) : (
                <HumanNoteItem
                  key={item.entry_id}
                  availableReactions={availableReactions}
                  continuesAbove={grouping[index].continuesAbove}
                  incidentId={incidentId}
                  item={item}
                />
              ),
            )}
          </ol>
        )}
      </div>

      {/*
       * Anchored below the transcript rather than above it. A composer at the
       * top reads as "post an update"; at the bottom, after what has already
       * been said, it reads as answering — the difference between a feed and a
       * room.
       */}
      {canCompose ? (
        <div data-incident-composer="true" className="shrink-0">
          <ActivityComposer incidentId={incidentId} people={people} />
        </div>
      ) : null}
    </section>
  );
}
