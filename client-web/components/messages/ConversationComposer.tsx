"use client";

import { useId, useRef, useState, type ReactNode } from "react";
import { ArrowUp, Paperclip, X } from "lucide-react";
import { GifSearchPanel } from "@/components/messages/GifSearchPanel";
import { IconButton } from "@/components/ui/Button";
import { ToggleButton } from "@/components/ui/ToggleButton";

export function ConversationComposer({
  attachmentLabel,
  attachmentRemoveLabel,
  attachmentRejectedText,
  allowAttachments = false,
  error,
  gifLabel,
  gifText,
  inputLabel,
  onChange,
  onSend,
  pending,
  placeholder,
  sendLabel,
  status,
}: {
  attachmentLabel?: string;
  attachmentRemoveLabel?: string;
  attachmentRejectedText?: string;
  allowAttachments?: boolean;
  error?: ReactNode;
  gifLabel: string;
  gifText: string;
  inputLabel: string;
  onChange?: () => void;
  onSend: (content: string, onSuccess: () => void, files?: File[]) => void;
  pending: boolean;
  placeholder: string;
  sendLabel: string;
  status?: ReactNode;
}) {
  const inputId = useId();
  const fileInputId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [content, setContent] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [attachmentError, setAttachmentError] = useState("");
  const [showGifPanel, setShowGifPanel] = useState(false);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const message = content.trim();
    if (!message && files.length === 0) return;
    onSend(
      message,
      () => {
        setContent("");
        setFiles([]);
        if (fileInputRef.current) fileInputRef.current.value = "";
      },
      files,
    );
  };

  return (
    <div className="px-4 pt-2 pb-4" data-conversation-composer="true">
      {showGifPanel ? (
        <GifSearchPanel
          disabled={pending}
          onClose={() => setShowGifPanel(false)}
          onSelect={(url) => onSend(`giphy:${url}`, () => setShowGifPanel(false))}
        />
      ) : null}

      {error ? <div className="mb-2">{error}</div> : null}
      {attachmentError ? (
        <p className="text-sev-critical mb-2 text-xs" role="alert">
          {attachmentError}
        </p>
      ) : null}

      <form
        onSubmit={submit}
        className="border-border bg-panel/55 focus-within:border-gold/40 rounded-xl border p-2 shadow-sm transition-colors"
      >
        {files.length > 0 ? (
          <ul className="mb-2 flex flex-wrap gap-1">
            {files.map((file, index) => (
              <li
                key={`${file.name}-${file.size}-${index}`}
                className="bg-panel-2 text-muted flex max-w-full items-center gap-1 rounded-md px-2 py-1 text-xs"
              >
                <span className="truncate">{file.name}</span>
                <IconButton
                  type="button"
                  label={attachmentRemoveLabel ?? "Remove attachment"}
                  size="xs"
                  variant="ghost"
                  onClick={() => setFiles((current) => current.filter((_, item) => item !== index))}
                >
                  <X className="h-3 w-3" aria-hidden="true" />
                </IconButton>
              </li>
            ))}
          </ul>
        ) : null}

        <label htmlFor={inputId} className="block">
          <span className="sr-only">{inputLabel}</span>
          <input
            id={inputId}
            type="text"
            value={content}
            onChange={(event) => {
              setContent(event.target.value);
              onChange?.();
            }}
            placeholder={placeholder}
            className="text-text placeholder:text-muted h-9 w-full min-w-0 bg-transparent px-2 text-sm outline-none"
          />
        </label>

        <div className="mt-1 flex items-center justify-between gap-3">
          <div className="flex items-center gap-1">
            <ToggleButton
              className={showGifPanel ? "border-border bg-panel-2 text-text" : undefined}
              size="sm"
              variant="ghost"
              pressed={showGifPanel}
              onClick={() => setShowGifPanel((current) => !current)}
              aria-label={gifLabel}
            >
              {gifText}
            </ToggleButton>
            {allowAttachments ? (
              <>
                <input
                  ref={fileInputRef}
                  id={fileInputId}
                  type="file"
                  aria-label={attachmentLabel ?? "Attach files"}
                  multiple
                  hidden
                  accept="image/jpeg,image/png,image/gif,image/webp,application/pdf,application/json,application/zip,application/gzip,application/octet-stream,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.openxmlformats-officedocument.presentationml.presentation,text/plain,text/csv,text/markdown,text/yaml,.log,.md,.yaml,.yml"
                  onChange={(event) => {
                    const selected = Array.from(event.target.files ?? []);
                    const totalBytes = selected.reduce((total, file) => total + file.size, 0);
                    if (
                      selected.length > 4 ||
                      selected.some((file) => file.size === 0 || file.size > 5 * 1024 * 1024) ||
                      totalBytes > 10 * 1024 * 1024
                    ) {
                      setAttachmentError(attachmentRejectedText ?? "Attachments are invalid");
                      event.target.value = "";
                      return;
                    }
                    setAttachmentError("");
                    setFiles(selected);
                  }}
                />
                <IconButton
                  type="button"
                  label={attachmentLabel ?? "Attach files"}
                  size="sm"
                  variant="ghost"
                  onClick={() => fileInputRef.current?.click()}
                >
                  <Paperclip className="h-4 w-4" aria-hidden="true" />
                </IconButton>
              </>
            ) : null}
          </div>
          <IconButton
            className="rounded-full"
            type="submit"
            label={sendLabel}
            size="sm"
            variant="primary"
            disabled={!content.trim() && files.length === 0}
            loading={pending}
          >
            <ArrowUp className="h-4 w-4" strokeWidth={2} aria-hidden="true" />
          </IconButton>
        </div>
      </form>

      {status ? <div className="mt-2">{status}</div> : null}
    </div>
  );
}
