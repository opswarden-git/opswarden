import { Skeleton } from "@/components/ui/Skeleton";
import { cn } from "@/lib/utils";
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

      <aside className="bg-panel/25 border-border hidden border-l p-4 lg:block">
        {context === "incident" ? (
          <>
            <div className="space-y-4">
              {[0, 1].map((index) => (
                <div key={index} className="flex items-center justify-between gap-4">
                  <Skeleton className="h-3 w-16" />
                  <Skeleton className="h-5 w-20 rounded-full" />
                </div>
              ))}
              <Skeleton className="h-3 w-16" />
              <div className="flex gap-2">
                <Skeleton className="h-9 flex-1" />
                <Skeleton className="h-8 w-8" />
              </div>
            </div>
            <div className="border-border mt-4 space-y-2 border-t pt-4">
              <Skeleton className="h-3 w-14" />
              <Skeleton className="h-9 w-full" />
              <Skeleton className="h-9 w-full" />
            </div>
          </>
        ) : null}
        <div
          className={cn("space-y-3", context === "incident" && "border-border mt-4 border-t pt-4")}
        >
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
      </aside>
    </div>
  );
}
