import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface PageHeaderProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  context?: ReactNode;
  title: ReactNode;
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
  return (
    <header
      className={cn(
        "flex min-w-0 flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
        className,
      )}
      {...props}
    >
      <div className="min-w-0 space-y-2">
        {context ? (
          <div className="text-gold text-xs font-semibold tracking-wide uppercase">{context}</div>
        ) : null}
        <h1 className="text-text text-3xl font-semibold tracking-[-0.025em]">{title}</h1>
        {description ? (
          <p className="text-muted max-w-3xl text-sm leading-6">{description}</p>
        ) : null}
        {metadata ? <div className="text-muted text-sm">{metadata}</div> : null}
      </div>

      {actions ? (
        <div className="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">{actions}</div>
      ) : null}
    </header>
  );
}
