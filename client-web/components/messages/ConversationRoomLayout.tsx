"use client";

import React from "react";
import { cn } from "@/lib/utils";

export function ConversationRoomLayout({
  header,
  transcript,
  composer,
  sidebar,
  typingNotice,
  errorNotice,
  className,
}: {
  header?: React.ReactNode;
  transcript: React.ReactNode;
  composer: React.ReactNode;
  sidebar?: React.ReactNode;
  typingNotice?: React.ReactNode;
  errorNotice?: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn("grid h-full min-h-0 min-w-0 flex-1 grid-rows-[auto_minmax(0,1fr)_auto]", className)}
      data-conversation-room-layout="true"
    >
      {header ? <header className="shrink-0 border-b border-border">{header}</header> : null}

      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[minmax(0,1fr)_auto]">
        <main className="flex min-h-0 min-w-0 flex-col overflow-hidden">
          {errorNotice ? <div className="p-4 pb-0">{errorNotice}</div> : null}
          <div className="min-h-0 flex-1 overflow-y-auto">{transcript}</div>
          {typingNotice ? <div className="px-4 py-1 text-xs text-muted">{typingNotice}</div> : null}
        </main>
        {sidebar ? <aside className="hidden border-l border-border lg:block w-72 shrink-0">{sidebar}</aside> : null}
      </div>

      <footer className="shrink-0 border-t border-border">{composer}</footer>
    </div>
  );
}
