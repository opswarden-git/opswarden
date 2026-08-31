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
      className={cn(
        "grid h-full min-h-0 min-w-0 flex-1 grid-rows-[auto_minmax(0,1fr)_auto]",
        className,
      )}
      data-conversation-room-layout="true"
    >
      {header ? <header className="border-border shrink-0 border-b">{header}</header> : null}

      <div className="grid min-h-0 min-w-0 flex-1 grid-cols-1 overflow-hidden lg:grid-cols-[minmax(0,1fr)_auto]">
        <main className="flex min-h-0 min-w-0 flex-col overflow-hidden">
          {errorNotice ? <div className="p-4 pb-0">{errorNotice}</div> : null}
          <div className="min-h-0 flex-1 overflow-y-auto">{transcript}</div>
          {typingNotice ? <div className="text-muted px-4 py-1 text-xs">{typingNotice}</div> : null}
        </main>
        {sidebar ? (
          <aside className="border-border hidden w-72 shrink-0 border-l lg:block">{sidebar}</aside>
        ) : null}
      </div>

      <footer className="border-border shrink-0 border-t">{composer}</footer>
    </div>
  );
}
