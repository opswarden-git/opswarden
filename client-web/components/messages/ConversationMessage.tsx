"use client";

import { useState } from "react";
import { Download, FileText, Pencil, SmilePlus } from "lucide-react";
import { useLocale } from "next-intl";
import { Button, IconButton } from "@/components/ui/Button";
import { ReactionToggle } from "@/components/ui/ReactionToggle";
import { giphyEntryUrl } from "@/lib/queries/gifs";
import { cn } from "@/lib/utils";

export interface ConversationReaction {
  emoji: string;
  count: number;
  reacted: boolean;
}

export interface ConversationAttachment {
  id: string;
  fileName: string;
  sizeBytes: number;
}

interface ConversationMessageLabels {
  addReaction?: string;
  cancel: string;
  downloadFailed: string;
  edit: string;
  edited: string;
  editMessage: string;
  gifAlt: string;
  save: string;
}

interface ConversationMessageProps {
  attachments?: ConversationAttachment[];
  authorLabel: string;
  availableReactions?: string[];
  content: string;
  continuesAbove: boolean;
  createdAt: string;
  editedAt: string | null;
  labels: ConversationMessageLabels;
  mine: boolean;
  reactions?: ConversationReaction[];
  surface: "direct" | "incident";
  editError?: string;
  editPending?: boolean;
  reactionPending?: boolean;
  onDownload?: (attachmentId: string) => Promise<void>;
  onEdit?: (content: string, onSuccess: () => void) => void;
  onResetEdit?: () => void;
  onToggleReaction?: (emoji: string) => void;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.ceil(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ConversationMessage({
  attachments = [],
  authorLabel,
  availableReactions = [],
  content,
  continuesAbove,
  createdAt,
  editedAt,
  labels,
  mine,
  reactions = [],
  surface,
  editError,
  editPending = false,
  reactionPending = false,
  onDownload,
  onEdit,
  onResetEdit,
  onToggleReaction,
}: ConversationMessageProps) {
  const locale = useLocale();
  const [editing, setEditing] = useState(false);
  const [picking, setPicking] = useState(false);
  const [draft, setDraft] = useState(content);
  const [downloadError, setDownloadError] = useState("");
  const gifUrl = giphyEntryUrl(content);
  const canEdit = mine && !!content && !gifUrl && !!onEdit;
  const present = reactions.filter((reaction) => reaction.count > 0);
  const missing = availableReactions.filter(
    (emoji) => !present.some((reaction) => reaction.emoji === emoji),
  );
  const ownership = mine ? "current" : "peer";
  const surfaceData =
    surface === "direct"
      ? {
          "data-direct-message-owner": ownership,
          "data-message-continues-above": continuesAbove ? "true" : undefined,
        }
      : {
          "data-note-owner": ownership,
          "data-note-continues-above": continuesAbove ? "true" : undefined,
        };

  return (
    <li
      {...surfaceData}
      data-conversation-message="true"
      className={cn(
        "group flex w-full px-4",
        mine ? "justify-end" : "justify-start",
        continuesAbove ? "mt-1" : "mt-5 first:mt-0",
      )}
    >
      <div className={cn("flex max-w-[78%] min-w-0 flex-col", mine ? "items-end" : "items-start")}>
        {!continuesAbove && !mine ? (
          <span className="text-muted mb-1 px-1 text-xs font-medium">{authorLabel}</span>
        ) : null}
        <div className={cn("relative max-w-full", editing && "w-[min(32rem,70vw)]")}>
          {!editing ? (
            <div
              data-conversation-actions="true"
              className={cn(
                "absolute top-1 z-10 flex items-center gap-0.5 transition-opacity",
                mine ? "right-full mr-1" : "left-full ml-1",
                picking
                  ? "opacity-100"
                  : "pointer-events-none opacity-0 group-hover:pointer-events-auto group-hover:opacity-100 focus-within:pointer-events-auto focus-within:opacity-100",
              )}
            >
              {onToggleReaction && labels.addReaction ? (
                <IconButton
                  className="text-muted hover:text-text hover:bg-transparent"
                  label={labels.addReaction}
                  size="sm"
                  variant="ghost"
                  onClick={() => setPicking((current) => !current)}
                >
                  <SmilePlus className="h-3.5 w-3.5" aria-hidden="true" />
                </IconButton>
              ) : null}
              {canEdit ? (
                <IconButton
                  className="text-muted hover:text-text hover:bg-transparent"
                  label={labels.edit}
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    onResetEdit?.();
                    setDraft(content);
                    setEditing(true);
                  }}
                >
                  <Pencil className="h-3.5 w-3.5" aria-hidden="true" />
                </IconButton>
              ) : null}
            </div>
          ) : null}
          <div
            className={cn(
              "max-w-full border px-3 py-2 shadow-sm",
              editing && "w-full",
              mine
                ? "border-gold bg-gold rounded-2xl rounded-br-md"
                : "bg-panel-2 border-border rounded-2xl rounded-bl-md",
            )}
          >
            {editing ? (
              <div className="w-full space-y-3">
                <label>
                  <span className="sr-only">{labels.editMessage}</span>
                  <textarea
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    rows={3}
                    className="text-bg placeholder:text-bg/60 min-h-24 w-full resize-y border-0 bg-transparent p-0 text-sm leading-6 outline-none"
                  />
                </label>
                {editError ? (
                  <p className="text-sev-critical text-xs" role="alert">
                    {editError}
                  </p>
                ) : null}
                <div className="flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-bg hover:bg-bg/10 hover:text-bg"
                    onClick={() => setEditing(false)}
                  >
                    {labels.cancel}
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="border-bg/20 bg-bg text-text hover:bg-bg/85"
                    disabled={!draft.trim()}
                    loading={editPending}
                    onClick={() => onEdit?.(draft.trim(), () => setEditing(false))}
                  >
                    {labels.save}
                  </Button>
                </div>
              </div>
            ) : (
              <>
                {attachments.length > 0 ? (
                  <ul className="space-y-1">
                    {attachments.map((attachment) => (
                      <li key={attachment.id}>
                        <button
                          type="button"
                          className={cn(
                            "flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left transition-colors",
                            mine
                              ? "bg-bg/15 text-bg hover:bg-bg/25"
                              : "bg-panel text-text hover:bg-panel/70",
                          )}
                          onClick={async () => {
                            setDownloadError("");
                            try {
                              await onDownload?.(attachment.id);
                            } catch {
                              setDownloadError(labels.downloadFailed);
                            }
                          }}
                        >
                          <FileText className="h-4 w-4 shrink-0" aria-hidden="true" />
                          <span className="min-w-0 flex-1">
                            <span className="block truncate text-xs font-medium">
                              {attachment.fileName}
                            </span>
                            <span className="block text-[10px] opacity-70">
                              {formatBytes(attachment.sizeBytes)}
                            </span>
                          </span>
                          <Download className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                        </button>
                      </li>
                    ))}
                  </ul>
                ) : null}
                {downloadError ? (
                  <p
                    className={cn("mt-1 text-xs", mine ? "text-bg" : "text-sev-critical")}
                    role="alert"
                  >
                    {downloadError}
                  </p>
                ) : null}
                {gifUrl ? (
                  // External GIPHY media is selected by the user.
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={gifUrl}
                    alt={labels.gifAlt}
                    loading="lazy"
                    className={cn("max-h-72 max-w-full rounded-xl", attachments.length && "mt-2")}
                  />
                ) : content ? (
                  <p
                    className={cn(
                      "text-sm leading-6 break-words whitespace-pre-wrap",
                      mine ? "text-bg" : "text-text",
                      attachments.length && "mt-2",
                    )}
                  >
                    {content}
                  </p>
                ) : null}
              </>
            )}
          </div>
        </div>
        <div className={cn("mt-1 flex items-center gap-2 px-1", mine && "flex-row-reverse")}>
          <time
            className="text-muted-2 text-[11px]"
            dateTime={createdAt}
            title={new Intl.DateTimeFormat(locale, {
              dateStyle: "medium",
              timeStyle: "short",
            }).format(new Date(createdAt))}
          >
            {new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(new Date(createdAt))}
            {editedAt ? ` · ${labels.edited}` : ""}
          </time>
          {onToggleReaction && (present.length > 0 || picking) && !editing ? (
            <div className="flex flex-wrap items-center gap-1">
              {[
                ...present,
                ...(picking ? missing.map((emoji) => ({ emoji, count: 0, reacted: false })) : []),
              ].map((reaction) => (
                <ReactionToggle
                  key={reaction.emoji}
                  emoji={reaction.emoji}
                  count={reaction.count}
                  label={`${reaction.emoji} (${reaction.count})`}
                  pressed={reaction.reacted}
                  loading={reactionPending}
                  onClick={() => {
                    onToggleReaction(reaction.emoji);
                    if (reaction.count === 0) setPicking(false);
                  }}
                />
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </li>
  );
}
