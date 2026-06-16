import React, { useState, useRef } from "react";
import { useTimeline, useAddTimelineEntry } from "@/lib/queries/incidents";
import { useWsStore } from "@/lib/ws";
import { Send, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";

export function Timeline({ incidentId }: { incidentId: string }) {
  const { data, isLoading, error } = useTimeline(incidentId);
  const addEntry = useAddTimelineEntry();
  const [content, setContent] = useState("");
  const t = useTranslations("Incidents");

  const sendJson = useWsStore((s) => s.sendJson);
  const typingUsers = useWsStore((s) => s.typingUsers);
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

  if (isLoading) return <div className="text-muted animate-pulse p-4 text-sm">Loading logs...</div>;
  if (error) return <div className="text-sev-critical p-4 text-sm">Failed to load logs.</div>;

  return (
    <div className="flex h-full flex-col">
      <div className="surface-subtle border-border border-b p-4">
        <h2 className="text-text flex items-center gap-2 text-sm font-bold">
          <Terminal className="text-gold h-4 w-4" />
          Operator Log
        </h2>
      </div>

      <div className="flex flex-1 flex-col-reverse space-y-4 overflow-y-auto p-4">
        {/* We reverse the flex direction to keep the latest messages at the bottom if we mapped it backwards, 
            but the original was top-down. Let's just use regular flex and scroll down, or display them normally.
            Usually logs are top-to-bottom so the newest is at the bottom. 
            Wait, data.entries is likely sorted newest first or oldest first. 
            Let's keep original order but style it better. */}
        <div className="space-y-4">
          {data?.entries.length === 0 ? (
            <div className="text-muted p-4 text-center text-sm">No entries yet.</div>
          ) : (
            data?.entries.map((entry) => (
              <div key={entry.id} className="surface-subtle border-border rounded-md border p-4">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-text text-xs font-medium">
                    {entry.author_id.split("-")[0]}
                  </span>
                  <span className="text-muted/60 text-[10px]">
                    {new Date(entry.created_at).toLocaleTimeString()}
                  </span>
                </div>
                <p className="text-text text-sm leading-relaxed whitespace-pre-wrap">
                  {entry.content}
                </p>
              </div>
            ))
          )}
        </div>
      </div>

      {typingUsers.length > 0 && (
        <div className="text-gold/80 animate-pulse px-4 py-1 text-xs">
          {typingUsers.length === 1
            ? `${typingUsers[0].split("-")[0]} is typing...`
            : `${typingUsers.length} operators are typing...`}
        </div>
      )}

      <div className="surface-subtle border-border border-t p-4">
        <form onSubmit={handleSubmit} className="flex gap-2">
          <input
            type="text"
            value={content}
            onChange={handleContentChange}
            placeholder="Type command or log entry..."
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
