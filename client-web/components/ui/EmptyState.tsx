import React, { ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface EmptyStateProps {
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({
  title,
  description,
  icon,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div className={cn("surface rounded-md p-12 text-center", className)}>
      {icon ? (
        <div className="text-muted mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-surface-subtle border border-border">
          {icon}
        </div>
      ) : null}
      <h3 className="text-text font-semibold">{title}</h3>
      {description ? (
        <p className="text-muted mx-auto mt-1 max-w-md text-sm">{description}</p>
      ) : null}
      {action ? <div className="mt-4 flex justify-center">{action}</div> : null}
    </div>
  );
}
