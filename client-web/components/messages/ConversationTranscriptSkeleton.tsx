import { Skeleton } from "@/components/ui/Skeleton";
import { cn } from "@/lib/utils";

export function ConversationTranscriptSkeleton({
  className,
  label,
  systemEvents = false,
}: {
  className?: string;
  label: string;
  systemEvents?: boolean;
}) {
  return (
    <div
      aria-busy="true"
      aria-label={label}
      className={cn("flex min-h-0 flex-col justify-end py-4", className)}
      data-testid="conversation-transcript-skeleton"
    >
      <div className="flex items-center gap-3 px-4 py-3">
        <Skeleton className="h-px flex-1 rounded-none" />
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-px flex-1 rounded-none" />
      </div>
      <div className="space-y-3 px-4">
        <div className="flex justify-start">
          <Skeleton className="h-14 w-[58%] rounded-2xl" />
        </div>
        {systemEvents ? (
          <div className="flex justify-center py-1">
            <Skeleton className="h-3 w-48" />
          </div>
        ) : null}
        <div className="flex justify-end">
          <Skeleton className="h-12 w-[48%] rounded-2xl" />
        </div>
        <div className="flex justify-start">
          <Skeleton className="h-16 w-[52%] rounded-2xl" />
        </div>
      </div>
    </div>
  );
}
