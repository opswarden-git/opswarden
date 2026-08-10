"use client";

import React, { useState } from "react";
import { Pencil, SmilePlus } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import {
  type IncidentActivityItem,
  type TimelineReaction,
  useEditTimelineEntry,
  useToggleTimelineReaction,
} from "@/lib/queries/incidents";
import { useAuthStore } from "@/store/auth";
import { giphyEntryUrl } from "@/lib/queries/gifs";
import { Button, IconButton } from "@/components/ui/Button";
import { ReactionToggle } from "@/components/ui/ReactionToggle";
import { cn } from "@/lib/utils";

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

export function HumanNoteItem({
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
