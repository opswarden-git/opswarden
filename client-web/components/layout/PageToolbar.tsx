import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

/** Shared shell for search, filters and sort controls directly above a list. */
export function PageToolbar({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "surface-subtle border-border flex flex-col gap-3 rounded-md border p-3 lg:flex-row lg:items-center",
        className,
      )}
      {...props}
    />
  );
}
