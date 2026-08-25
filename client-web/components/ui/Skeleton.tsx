import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

/** Neutral loading geometry. The surrounding component owns the final layout. */
export function Skeleton({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      aria-hidden="true"
      className={cn("bg-panel-2 animate-pulse rounded", className)}
      {...props}
    />
  );
}
