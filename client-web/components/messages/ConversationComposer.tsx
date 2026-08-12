"use client";

import { useId, useState, type ReactNode } from "react";
import { ArrowUp } from "lucide-react";
import { GifSearchPanel } from "@/components/messages/GifSearchPanel";
import { IconButton } from "@/components/ui/Button";
import { ToggleButton } from "@/components/ui/ToggleButton";

export function ConversationComposer({
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
  error?: ReactNode;
  gifLabel: string;
  gifText: string;
  inputLabel: string;
  onChange?: () => void;
  onSend: (content: string, onSuccess: () => void) => void;
  pending: boolean;
  placeholder: string;
  sendLabel: string;
  status?: ReactNode;
}) {
  const inputId = useId();
  const [content, setContent] = useState("");
  const [showGifPanel, setShowGifPanel] = useState(false);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const message = content.trim();
    if (!message) return;
    onSend(message, () => setContent(""));
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

      <form
        onSubmit={submit}
        className="border-border bg-panel/55 focus-within:border-gold/40 rounded-xl border p-2 shadow-sm transition-colors"
      >
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
          <ToggleButton
            size="sm"
            variant="ghost"
            pressed={showGifPanel}
            onClick={() => setShowGifPanel((current) => !current)}
            aria-label={gifLabel}
          >
            {gifText}
          </ToggleButton>
          <IconButton
            type="submit"
            label={sendLabel}
            size="sm"
            variant="primary"
            disabled={!content.trim()}
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
