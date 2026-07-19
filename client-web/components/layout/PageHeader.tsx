import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface PageHeaderProps extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  title: ReactNode;
  description?: ReactNode;
  metadata?: ReactNode;
  actions?: ReactNode;
}

/** A predictable page heading with optional context and actions. */
export function PageHeader({
  actions,
  className,
  description,
  metadata,
  title,
  ...props
}: PageHeaderProps) {
  return (
    <header
      className={cn(
        "flex min-w-0 flex-col gap-4 sm:flex-row sm:items-start sm:justify-between",
        className,
      )}
      {...props}
    >
      <div className="min-w-0 space-y-1.5">
        <h1 className="text-text text-2xl font-bold tracking-tight">{title}</h1>
        {description ? <p className="text-muted max-w-3xl text-sm">{description}</p> : null}
        {metadata ? <div className="text-muted text-sm">{metadata}</div> : null}
      </div>

      {actions ? (
        <div className="flex shrink-0 flex-wrap items-center gap-2 sm:justify-end">{actions}</div>
      ) : null}
    </header>
  );
}
