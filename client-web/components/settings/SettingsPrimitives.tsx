import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function IdentityHeader({
  action,
  bordered = true,
  mark,
  title,
  subtitle,
  markInteractive = false,
}: {
  action?: ReactNode;
  bordered?: boolean;
  mark: ReactNode;
  title: ReactNode;
  subtitle?: ReactNode;
  markInteractive?: boolean;
}) {
  return (
    <header
      className={cn(
        "flex min-w-0 flex-wrap items-center gap-4",
        bordered && "border-border border-b pb-6",
      )}
    >
      <div className="flex min-w-0 flex-1 items-center gap-4">
        <span
          className="surface-subtle text-text border-border flex h-14 w-14 shrink-0 items-center justify-center rounded-full border text-sm font-semibold tracking-wide"
          aria-hidden={markInteractive ? undefined : "true"}
        >
          {mark}
        </span>
        <div className="min-w-0">
          <h2 className="text-text truncate text-lg font-semibold tracking-tight">{title}</h2>
          {subtitle ? <div className="text-muted mt-1 text-sm">{subtitle}</div> : null}
        </div>
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </header>
  );
}

export function SettingsSection({
  action,
  children,
  className,
  title,
}: {
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  title: ReactNode;
}) {
  return (
    <section className={cn("space-y-3", className)}>
      <div className="flex min-h-9 flex-wrap items-center justify-between gap-3">
        <h2 className="text-text font-semibold">{title}</h2>
        {action}
      </div>
      <div className="divide-border-muted divide-y">{children}</div>
    </section>
  );
}

export function SettingsRow({
  action,
  children,
  className,
  label,
}: {
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  label: ReactNode;
}) {
  return (
    <div
      className={cn(
        "grid min-h-16 gap-2 py-4 sm:grid-cols-[minmax(10rem,0.55fr)_minmax(0,1fr)_auto] sm:items-center sm:gap-5",
        className,
      )}
    >
      <div className="text-muted text-sm font-medium">{label}</div>
      <div className="text-text min-w-0 text-sm">{children}</div>
      {action ? <div className="justify-self-start sm:justify-self-end">{action}</div> : null}
    </div>
  );
}
