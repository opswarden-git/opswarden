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
  ...props
}: PageHeaderProps) {
  const hasSummary = context || title || description || metadata;
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
          <div className="min-w-0 space-y-2">
            {context ? (
              <div className="text-gold text-xs font-semibold tracking-wide uppercase">
                {context}
              </div>
            ) : null}
            {title ? (
              <h1 className="text-text text-3xl font-semibold tracking-[-0.025em]">{title}</h1>
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
