import React, { useState } from "react";
import { useTimeline, useAddTimelineEntry } from "@/lib/queries/incidents";
import { Send, Terminal } from "lucide-react";
import { useTranslations } from "next-intl";

export function Timeline({ incidentId }: { incidentId: string }) {
  const { data, isLoading, error } = useTimeline(incidentId);
  const addEntry = useAddTimelineEntry();
  const [content, setContent] = useState("");
  const t = useTranslations("Incidents");

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

  if (isLoading) return <div className="text-muted animate-pulse p-4">{t("loadingTimeline")}</div>;
  if (error) return <div className="p-4 text-red-500">{t("failedToLoadTimeline")}</div>;

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-white/5 bg-white/5 p-4">
        <h2 className="text-text flex items-center gap-2 font-sans text-lg font-bold">
          <Terminal className="text-gold h-5 w-5" />
          {t("operatorTimeline")}
        </h2>
      </div>

      <div className="flex-1 space-y-4 overflow-y-auto p-4">
        {data?.entries.length === 0 ? (
          <div className="text-muted p-4 text-center text-sm">{t("noEntriesYet")}</div>
        ) : (
          data?.entries.map((entry) => (
            <div key={entry.id} className="rounded-md border border-white/5 bg-black/30 p-3">
              <div className="mb-1 flex items-center justify-between">
                <span className="text-gold font-mono text-xs">{entry.author_id.split("-")[0]}</span>
                <span className="text-muted/60 font-mono text-[10px]">
                  {new Date(entry.created_at).toLocaleTimeString()}
                </span>
              </div>
              <p className="text-text font-mono text-sm whitespace-pre-wrap">{entry.content}</p>
            </div>
          ))
        )}
      </div>

      <div className="border-t border-white/5 bg-white/5 p-4">
        <form onSubmit={handleSubmit} className="flex gap-2">
          <input
            type="text"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t("logEventPlaceholder")}
            className="focus:border-gold flex-1 rounded-md border border-white/10 bg-black/50 px-3 py-2 font-mono text-sm text-white focus:outline-none"
          />
          <button
            type="submit"
            disabled={addEntry.isPending || !content.trim()}
            className="bg-gold hover:bg-gold-hover text-bg disabled:bg-gold/50 disabled:text-bg/50 flex items-center justify-center rounded-md px-4 py-2 transition-colors"
          >
            <Send className="h-4 w-4" />
          </button>
        </form>
      </div>
    </div>
  );
}
