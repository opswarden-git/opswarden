import React, { useState, useRef } from "react";
import {
  useTimeline,
  useAddTimelineEntry,
  useEditTimelineEntry,
  useToggleTimelineReaction,
  type TimelineEntry,
} from "@/lib/queries/incidents";
import { useWsStore, useTypingUsers } from "@/lib/ws";
import { useAuthStore } from "@/store/auth";
import { Pencil, Send, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";
import { cn } from "@/lib/utils";

const PRESET_REACTIONS = ["👍", "👀", "✅", "🚨"];

function TimelineEntryItem({
  entry,
  incidentId,
  currentUserId,
}: {
  entry: TimelineEntry;
  incidentId: string;
  currentUserId?: string;
}) {
  const t = useTranslations("Incidents");
  const tErr = useTranslations("errors");
  const editEntry = useEditTimelineEntry();
  const toggleReaction = useToggleTimelineReaction();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(entry.content);

  const isAuthor = !!currentUserId && entry.author_id === currentUserId;
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

  // Preset emojis plus any non-preset emoji that already carries a reaction.
  const extraEmojis = entry.reactions
    .map((r) => r.emoji)
    .filter((e) => !PRESET_REACTIONS.includes(e));
  const emojis = [...PRESET_REACTIONS, ...extraEmojis];

  return (
    <div className="surface-subtle border-border rounded-md border p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-text text-xs font-medium">{entry.author_id.split("-")[0]}</span>
        <div className="flex items-center gap-2">
          {entry.edited_at ? (
            <span className="text-muted/50 text-[10px] italic">{t("edited")}</span>
          ) : null}
          <span className="text-muted/60 text-[10px]">
            {new Date(entry.created_at).toLocaleTimeString()}
          </span>
          {isAuthor && !editing ? (
            <button
              type="button"
              onClick={startEdit}
              title={t("edit")}
              aria-label={t("edit")}
              className="text-muted hover:text-text transition-colors"
            >
              <Pencil className="h-3 w-3" />
            </button>
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
            <button
              type="button"
              onClick={() => setEditing(false)}
              className="ow-secondary h-8 rounded-md px-3 text-xs font-medium transition-colors"
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              onClick={saveEdit}
              disabled={editEntry.isPending || !draft.trim()}
              className="ow-primary h-8 rounded-md px-3 text-xs font-medium transition-colors disabled:opacity-50"
            >
              {t("save")}
            </button>
          </div>
        </div>
      ) : (
        <p className="text-text text-sm leading-relaxed whitespace-pre-wrap">{entry.content}</p>
      )}

      <div className="mt-2 flex flex-wrap items-center gap-1">
        {emojis.map((emoji) => {
          const r = entry.reactions.find((x) => x.emoji === emoji);
          const count = r?.count ?? 0;
          const reacted = r?.reacted ?? false;
          return (
            <button
              key={emoji}
              type="button"
              onClick={() => toggleReaction.mutate({ incidentId, entryId: entry.id, emoji })}
              disabled={toggleReaction.isPending}
              className={cn(
                "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-xs transition-colors disabled:opacity-50",
                reacted
                  ? "border-gold/40 bg-gold/10 text-text"
                  : "border-border text-muted hover:text-text hover:bg-white/[0.04]",
              )}
            >
              <span>{emoji}</span>
              {count > 0 ? <span className="tabular-nums">{count}</span> : null}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export function Timeline({ incidentId }: { incidentId: string }) {
  const { data, isLoading, error } = useTimeline(incidentId);
  const addEntry = useAddTimelineEntry();
  const [content, setContent] = useState("");
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
    return <div className="text-sev-critical p-4 text-sm">{t("failedToLoadTimeline")}</div>;

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

      <div className="surface-subtle border-border border-t p-4">
        <form onSubmit={handleSubmit} className="flex gap-2">
          <input
            type="text"
            value={content}
            onChange={handleContentChange}
            placeholder={t("logEventPlaceholder")}
            className="ow-input flex h-10 flex-1 rounded-md px-3 py-2 text-sm transition-colors"
          />
          <button
            type="submit"
            disabled={addEntry.isPending || !content.trim()}
            className="ow-primary flex h-10 w-10 shrink-0 items-center justify-center rounded-md transition-colors disabled:opacity-50"
          >
            <Send className="h-4 w-4" />
          </button>
        </form>
      </div>
    </div>
  );
}
