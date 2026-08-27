"use client";

import React, { type ReactNode, useEffect, useRef } from "react";
import { Button } from "@/components/ui/Button";

interface ConversationTranscriptProps<Item> {
  empty: ReactNode;
  error: ReactNode;
  getCreatedAt: (item: Item) => string;
  getId: (item: Item) => string;
  items: Item[];
  loading: ReactNode;
  locale: string;
  renderItem: (item: Item, index: number, continuesAbove: boolean) => ReactNode;
  surface: "direct" | "incident";
  continuesFromPrevious?: (item: Item, previous: Item | undefined) => boolean;
  isLoading?: boolean;
  hasError?: boolean;
  loadEarlier?: () => Promise<unknown>;
  loadEarlierLabel?: string;
  loadingEarlier?: boolean;
}

function localDayKey(value: string) {
  const date = new Date(value);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
}

export function ConversationTranscript<Item>({
  empty,
  error,
  getCreatedAt,
  getId,
  items,
  loading,
  locale,
  renderItem,
  surface,
  continuesFromPrevious,
  isLoading = false,
  hasError = false,
  loadEarlier,
  loadEarlierLabel,
  loadingEarlier = false,
}: ConversationTranscriptProps<Item>) {
  const transcriptRef = useRef<HTMLDivElement>(null);
  const isNearBottom = useRef(true);
  const hasPositioned = useRef(false);
  const newestId = items.at(-1) ? getId(items.at(-1) as Item) : undefined;

  useEffect(() => {
    const transcript = transcriptRef.current;
    if (transcript && (!hasPositioned.current || isNearBottom.current)) {
      transcript.scrollTop = transcript.scrollHeight;
      isNearBottom.current = true;
    }
    hasPositioned.current = true;
  }, [newestId]);

  const loadPreviousPage = async () => {
    if (!loadEarlier) return;
    const transcript = transcriptRef.current;
    const previousHeight = transcript?.scrollHeight ?? 0;
    const previousTop = transcript?.scrollTop ?? 0;
    await loadEarlier();
    requestAnimationFrame(() => {
      if (transcript) {
        transcript.scrollTop = previousTop + transcript.scrollHeight - previousHeight;
      }
    });
  };

  const surfaceData =
    surface === "direct"
      ? { "data-direct-message-transcript": "true" }
      : { "data-incident-transcript": "true" };

  return (
    <div
      ref={transcriptRef}
      {...surfaceData}
      className="min-h-0 flex-1 overflow-y-auto"
      onScroll={(event) => {
        const transcript = event.currentTarget;
        isNearBottom.current =
          transcript.scrollHeight - transcript.scrollTop - transcript.clientHeight <= 80;
      }}
    >
      <div
        className="flex min-h-full flex-col justify-end"
        data-conversation-content="true"
      >
        {loadEarlier ? (
          <div className="flex justify-center px-4 pt-3 pb-2">
            <Button size="sm" loading={loadingEarlier} onClick={() => void loadPreviousPage()}>
              {loadEarlierLabel}
            </Button>
          </div>
        ) : null}
        {isLoading ? (
          loading
        ) : hasError ? (
          error
        ) : items.length === 0 ? (
          empty
        ) : (
          <ol className="relative py-4">
            {items.map((item, index) => {
              const previous = items[index - 1];
              const createdAt = getCreatedAt(item);
              const showDay =
                !previous || localDayKey(getCreatedAt(previous)) !== localDayKey(createdAt);
              return (
                <React.Fragment key={getId(item)}>
                  {showDay ? (
                    <li className="flex items-center gap-3 px-4 py-3" aria-hidden="true">
                      <span className="bg-border h-px flex-1" />
                      <time
                        className="text-muted-2 text-[10px] font-medium tracking-wide uppercase"
                        dateTime={createdAt}
                      >
                        {new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(
                          new Date(createdAt),
                        )}
                      </time>
                      <span className="bg-border h-px flex-1" />
                    </li>
                  ) : null}
                  {renderItem(
                    item,
                    index,
                    !showDay && (continuesFromPrevious?.(item, previous) ?? false),
                  )}
                </React.Fragment>
              );
            })}
          </ol>
        )}
      </div>
    </div>
  );
}
