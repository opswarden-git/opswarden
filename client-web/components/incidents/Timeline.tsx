import React, { useState, useRef } from "react";
import {
  useTimeline,
  useAddTimelineEntry,
  useEditTimelineEntry,
  useToggleTimelineReaction,
  useAvailableReactions,
  type TimelineEntry,
} from "@/lib/queries/incidents";
import { useWsStore, useTypingUsers } from "@/lib/ws";
import { useAuthStore } from "@/store/auth";
import { giphyEntryUrl } from "@/lib/queries/gifs";
import { GifSearchPanel } from "./GifSearchPanel";
import { Pencil, Send, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ReactionToggle } from "@/components/ui/ReactionToggle";
import { Alert } from "@/components/ui/Alert";

function TimelineEntryItem({
  entry,
  incidentId,
  currentUserId,
  availableReactions,
}: {
  entry: TimelineEntry;
  incidentId: string;
  currentUserId?: string;
  availableReactions: string[];
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const editEntry = useEditTimelineEntry();
  const toggleReaction = useToggleTimelineReaction();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(entry.content);

  const isAuthor = !!currentUserId && entry.author_id === currentUserId;
  // A GIF entry renders as an image; only host-allowlisted giphy.com URLs pass.
  const gifUrl = giphyEntryUrl(entry.content);
  const errorText = (code: string) => (tErr.has(code) ? tErr(code) : t("actionFailed"));

  const startEdit = () => {
    editEntry.reset();
    setDraft(entry.content);
    setEditing(true);
  };
  const saveEdit = () => {
    const content = draft.trim();
    if (!content) return;
    editEntry.mutate(
      { incidentId, entryId: entry.id, content },
      { onSuccess: () => setEditing(false) },
    );
  };

  // Be defensive for stale React Query/HMR data loaded before reactions existed.
  const reactions = entry.reactions ?? [];
  // Preset emojis plus any non-preset emoji that already carries a reaction.
  const extraEmojis = reactions
    .map((reaction) => reaction.emoji)
    .filter((emoji) => !availableReactions.includes(emoji));
  const emojis = [...availableReactions, ...extraEmojis];

  return (
    <div className="surface-subtle border-border rounded-md border p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-text text-xs font-medium">
          {entry.author?.email ?? t("deletedUser")}
        </span>
        <div className="flex items-center gap-2">
          {entry.edited_at ? (
            <span className="text-muted/50 text-[10px] italic">{t("edited")}</span>
          ) : null}
          <span className="text-muted/60 text-[10px]">
            {new Date(entry.created_at).toLocaleTimeString()}
          </span>
          {isAuthor && !editing && !gifUrl ? (
            <IconButton onClick={startEdit} label={t("edit")} size="sm" variant="ghost">
              <Pencil className="h-3.5 w-3.5" />
            </IconButton>
          ) : null}
        </div>
      </div>

      {editing ? (
        <div className="space-y-2">
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={2}
            className="ow-input flex w-full rounded-md px-3 py-2 text-sm transition-colors"
          />
          {editEntry.error ? (
            <p className="text-sev-critical text-xs">{errorText(editEntry.error.message)}</p>
          ) : null}
          <div className="flex justify-end gap-2">
            <Button size="sm" onClick={() => setEditing(false)}>
              {t("cancel")}
            </Button>
            <Button
              size="sm"
              variant="primary"
              onClick={saveEdit}
              disabled={editEntry.isPending || !draft.trim()}
              loading={editEntry.isPending}
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
          className="max-h-56 max-w-full rounded-md"
        />
      ) : (
        <p className="text-text text-sm leading-relaxed whitespace-pre-wrap">{entry.content}</p>
      )}

      <div className="mt-2 flex flex-wrap items-center gap-1">
        {emojis.map((emoji) => {
          const r = reactions.find((x) => x.emoji === emoji);
          const count = r?.count ?? 0;
          const reacted = r?.reacted ?? false;
          return (
            <ReactionToggle
              key={emoji}
              emoji={emoji}
              count={count}
              label={`${emoji} (${count})`}
              pressed={reacted}
              onClick={() => toggleReaction.mutate({ incidentId, entryId: entry.id, emoji })}
              loading={toggleReaction.isPending}
            />
          );
        })}
      </div>
    </div>
  );
}

export function Timeline({ incidentId, canCompose }: { incidentId: string; canCompose: boolean }) {
  const { data, isLoading, error } = useTimeline(incidentId);
  const { data: availableReactions = [] } = useAvailableReactions();
  const addEntry = useAddTimelineEntry();
  const [content, setContent] = useState("");
  const [showGifPanel, setShowGifPanel] = useState(false);
  const t = useTranslations("Incidents");
  const currentUserId = useAuthStore((s) => s.user?.id);

  const sendJson = useWsStore((s) => s.sendJson);
  const typingUsers = useTypingUsers(incidentId);
  const lastTypingTime = useRef(0);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;
    addEntry.mutate(
      { incidentId, content: content.trim() },
      {
        onSuccess: () => setContent(""),
      },
    );
  };

  // Insert a GIF as a timeline entry using the `giphy:<url>` sentinel, reusing
  // the same mutation (and realtime fan-out) as a normal text entry.
  const handleSelectGif = (url: string) => {
    addEntry.mutate(
      { incidentId, content: `giphy:${url}` },
      { onSuccess: () => setShowGifPanel(false) },
    );
  };

  const handleContentChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setContent(e.target.value);
    const now = Date.now();
    if (now - lastTypingTime.current > 1500) {
      sendJson({ type: "status_typing", incident_id: incidentId });
      lastTypingTime.current = now;
    }
  };

  if (isLoading)
    return <div className="text-muted animate-pulse p-4 text-sm">{t("loadingTimeline")}</div>;
  if (error)
    return (
      <Alert tone="danger" className="m-4">
        {t("failedToLoadTimeline")}
      </Alert>
    );

  return (
    <div className="flex h-full flex-col">
      <div className="surface-subtle border-border border-b p-4">
        <h2 className="text-text flex items-center gap-2 text-sm font-bold">
          <Terminal className="text-gold h-4 w-4" />
          {t("operatorTimeline")}
        </h2>
      </div>

      <div className="flex flex-1 flex-col-reverse space-y-4 overflow-y-auto p-4">
        <div className="space-y-4">
          {data?.entries.length === 0 ? (
            <div className="text-muted p-4 text-center text-sm">{t("noEntriesYet")}</div>
          ) : (
            data?.entries.map((entry) => (
              <TimelineEntryItem
                key={entry.id}
                entry={entry}
                incidentId={incidentId}
                currentUserId={currentUserId}
                availableReactions={availableReactions}
              />
            ))
          )}
        </div>
      </div>

      {typingUsers.length > 0 && (
        <div className="text-gold/80 animate-pulse px-4 py-1 text-xs">
          {typingUsers.length === 1
            ? t("typingOne", { user: typingUsers[0].split("-")[0] })
            : t("typingMany", { count: typingUsers.length })}
        </div>
      )}

      {canCompose ? (
        <div className="surface-subtle border-border border-t p-4">
          {showGifPanel ? (
            <GifSearchPanel
              onSelect={handleSelectGif}
              onClose={() => setShowGifPanel(false)}
              disabled={addEntry.isPending}
            />
          ) : null}
          <form onSubmit={handleSubmit} className="flex gap-2">
            <ToggleButton
              size="lg"
              pressed={showGifPanel}
              onClick={() => setShowGifPanel((v) => !v)}
              aria-label={t("gifButton")}
            >
              GIF
            </ToggleButton>
            <input
              type="text"
              value={content}
              onChange={handleContentChange}
              placeholder={t("logEventPlaceholder")}
              className="ow-input flex h-10 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
            />
            <IconButton
              type="submit"
              label={t("send")}
              size="lg"
              variant="primary"
              disabled={addEntry.isPending || !content.trim()}
              loading={addEntry.isPending}
            >
              <Send className="h-4 w-4" />
            </IconButton>
          </form>
        </div>
      ) : null}
    </div>
  );
}
