import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export type PageContentState = "ready" | "loading" | "error" | "empty";

export interface PageContentProps extends Omit<HTMLAttributes<HTMLElement>, "children"> {
  state?: PageContentState;
  children?: ReactNode;
  loadingFallback?: ReactNode;
  errorFallback?: ReactNode;
  emptyFallback?: ReactNode;
}

/** Keeps the page shell stable while only the content region changes state. */
export function PageContent({
  children,
  className,
  emptyFallback = null,
  errorFallback = null,
  loadingFallback = null,
  state = "ready",
  ...props
}: PageContentProps) {
  const content = {
    ready: children,
    loading: loadingFallback,
    error: errorFallback,
    empty: emptyFallback,
  }[state];

  return (
    <section
      aria-busy={state === "loading" || undefined}
      className={cn("min-w-0", className)}
      data-state={state}
      {...props}
    >
      {content}
    </section>
  );
}
