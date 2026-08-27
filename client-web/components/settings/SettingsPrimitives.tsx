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
        "flex min-w-0 flex-wrap items-center gap-3",
        bordered && "border-border/40 mb-3 border-b pb-3",
      )}
    >
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <span
          className="surface-subtle text-text border-border/60 flex h-9 w-9 shrink-0 items-center justify-center rounded-full border text-xs font-semibold tracking-wide"
          aria-hidden={markInteractive ? undefined : "true"}
        >
          {mark}
        </span>
        <div className="min-w-0">
          <h2 className="text-text truncate text-sm leading-5 font-semibold tracking-tight">
            {title}
          </h2>
          {subtitle ? <div className="text-muted mt-0.5 text-xs">{subtitle}</div> : null}
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
    // A rule belongs between two topics, not between two facts about one. The
    // rows below a heading are the same subject — an identity, a preference —
    // so they group by proximity; the sections themselves are what a reader
    // needs help telling apart.
    <section
      className={cn(
        "[&:not(:first-child)]:border-border/40 space-y-1 [&:not(:first-child)]:mt-3 [&:not(:first-child)]:border-t [&:not(:first-child)]:pt-3",
        className,
      )}
    >
      <div className="flex min-h-7 flex-wrap items-center justify-between gap-2">
        <h2 className="text-text text-xs leading-5 font-semibold tracking-wider uppercase">
          {title}
        </h2>
        {action}
      </div>
      <div>{children}</div>
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
        "grid min-h-9 gap-2 py-1.5 sm:grid-cols-[minmax(8rem,0.35fr)_minmax(0,1fr)_auto] sm:items-center sm:gap-4",
        className,
      )}
    >
      <div className="text-muted text-xs font-medium">{label}</div>
      <div className="text-text min-w-0 text-sm">{children}</div>
      {action ? <div className="justify-self-start sm:justify-self-end">{action}</div> : null}
    </div>
  );
}
