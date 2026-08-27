"use client";

import { useSyncExternalStore, type HTMLAttributes, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { usePageActionsHost } from "./PageActionsRail";
import { cn } from "@/lib/utils";

const desktopRailQuery = "(min-width: 768px)";

function subscribeToDesktopRail(onChange: () => void) {
  const media = window.matchMedia?.(desktopRailQuery);
  if (!media) return () => undefined;
  media.addEventListener("change", onChange);
  return () => media.removeEventListener("change", onChange);
}

function readsDesktopRail() {
  return window.matchMedia?.(desktopRailQuery).matches ?? true;
}

export interface PageHeaderProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  context?: ReactNode;
  title?: ReactNode;
  titleAside?: ReactNode;
  description?: ReactNode;
  metadata?: ReactNode;
  actions?: ReactNode;
}

/** A predictable page heading with optional context and actions. */
export function PageHeader({
  actions,
  className,
  context,
  description,
  metadata,
  title,
  titleAside,
  ...props
}: PageHeaderProps) {
  const hasSummary = context || title || titleAside || description || metadata;
  const actionsHost = usePageActionsHost();
  const usesDesktopRail = useSyncExternalStore(
    subscribeToDesktopRail,
    readsDesktopRail,
    () => false,
  );

  const portaledActions =
    actions && actionsHost && usesDesktopRail
      ? createPortal(<>{actions}</>, actionsHost, "page-header-actions")
      : null;
  const inlineActions = actions && (!actionsHost || !usesDesktopRail) ? actions : null;

  if (!hasSummary && !inlineActions) return portaledActions;

  return (
    <>
      {portaledActions}
      <header
        className={cn(
          "flex min-w-0 flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
          !hasSummary && "sm:justify-end",
          className,
        )}
        {...props}
      >
        {hasSummary ? (
          <div className="min-w-0 flex-1 space-y-2">
            {context ? (
              <div className="text-gold text-xs font-semibold tracking-wide uppercase">
                {context}
              </div>
            ) : null}
            {title || titleAside ? (
              <div className="flex min-w-0 flex-wrap items-center justify-between gap-x-4 gap-y-2">
                {title ? (
                  <h1 className="text-text min-w-0 text-3xl font-semibold tracking-[-0.025em]">
                    {title}
                  </h1>
                ) : null}
                {titleAside ? <div className="shrink-0">{titleAside}</div> : null}
              </div>
            ) : null}
            {description ? (
              <p className="text-muted max-w-3xl text-sm leading-6">{description}</p>
            ) : null}
            {metadata ? <div className="text-muted text-sm">{metadata}</div> : null}
          </div>
        ) : null}

        {inlineActions ? (
          <div className="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">
            {inlineActions}
          </div>
        ) : null}
      </header>
    </>
  );
}
