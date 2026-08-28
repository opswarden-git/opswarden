import { Skeleton } from "@/components/ui/Skeleton";
import { ConversationTranscriptSkeleton } from "./ConversationTranscriptSkeleton";

export function ConversationRoomSkeleton({
  context = "members",
  label,
}: {
  context?: "incident" | "members";
  label: string;
}) {
  return (
    <div
      aria-busy="true"
      aria-label={label}
      className="border-border grid min-h-0 flex-1 grid-cols-1 overflow-hidden border-y lg:grid-cols-[minmax(0,1fr)_19rem] xl:grid-cols-[14rem_minmax(0,1fr)_19rem]"
      data-testid="conversation-room-skeleton"
    >
      <aside className="bg-panel/25 border-border hidden border-r xl:block">
        <div className="px-2 pt-3 pb-4">
          <div className="flex h-7 items-center px-2">
            <Skeleton className="h-3 w-24" />
          </div>
          <div className="mt-1 space-y-0.5">
            {[72, 88, 64, 80, 56].map((width, index) => (
              <div key={index} className="flex min-h-9 items-center gap-2 rounded px-2 py-1.5">
                <Skeleton className="h-1.5 w-1.5 shrink-0 rounded-full" />
                <Skeleton className="h-3" style={{ width: `${width}%` }} />
              </div>
            ))}
          </div>
        </div>
      </aside>

      <main className="flex min-h-0 min-w-0 flex-col">
        <ConversationTranscriptSkeleton
          className="min-h-0 flex-1"
          label={label}
          systemEvents={context === "incident"}
        />
        <div className="px-4 pt-2 pb-4">
          <div className="border-border bg-panel/55 rounded-xl border p-2 shadow-sm">
            <Skeleton className="h-9 w-40 max-w-full" />
            <div className="mt-1 flex items-center justify-between gap-3">
              <div className="flex items-center gap-1">
                <Skeleton className="h-8 w-10" />
                <Skeleton className="h-8 w-8" />
              </div>
              <Skeleton className="h-8 w-8 rounded-full" />
            </div>
          </div>
        </div>
      </main>

      <aside className="bg-panel/25 border-border hidden border-l lg:block">
        {context === "incident" ? (
          <div className="divide-border-muted divide-y p-2" data-testid="incident-context-skeleton">
            {[52, 60, 56, 48].map((width, index) => (
              <div key={index} className="flex h-6 items-center gap-1 px-1" aria-hidden="true">
                <Skeleton className="h-2.5 w-2.5 shrink-0" />
                <Skeleton className="h-2.5" style={{ width: `${width}px` }} />
              </div>
            ))}

            <section className="min-w-0" aria-hidden="true">
              <div className="flex h-6 items-center gap-1 px-1">
                <Skeleton className="h-2.5 w-2.5 shrink-0" />
                <Skeleton className="h-2.5 w-16" />
              </div>
              <div className="space-y-1 px-2 pt-1 pb-4">
                {[68, 76, 60].map((width, index) => (
                  <div key={index} className="flex min-h-12 items-center gap-2 px-2 py-2">
                    <Skeleton className="h-5 w-5 shrink-0" />
                    <div className="min-w-0 flex-1 space-y-1">
                      <Skeleton className="h-3" style={{ width: `${width}%` }} />
                      <Skeleton className="h-2.5 w-16" />
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>
        ) : (
          <div className="space-y-3 p-4">
            <Skeleton className="h-3 w-20" />
            {[0, 1, 2, 3].map((index) => (
              <div key={index} className="flex items-center gap-3">
                <Skeleton className="h-8 w-8 shrink-0 rounded-full" />
                <div className="min-w-0 flex-1 space-y-1.5">
                  <Skeleton className="h-3 w-3/4" />
                  <Skeleton className="h-3 w-20" />
                </div>
              </div>
            ))}
          </div>
        )}
      </aside>
    </div>
  );
}
