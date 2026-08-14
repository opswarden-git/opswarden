import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export type StatusBadgeTone = "neutral" | "info" | "warning" | "danger" | "success";

const toneClasses: Record<StatusBadgeTone, string> = {
  neutral: "bg-status-neutral",
  info: "bg-status-info",
  warning: "bg-status-warning",
  danger: "bg-status-danger",
  success: "bg-status-success",
};

/**
 * Opaque lifecycle panel. Color, icon and text are all mandatory signals.
 * Metadata, counters, presence indicators and filters deliberately use other
 * primitives so this grammar remains reserved for operational state.
 */
export function StatusBadge({
  children,
  className,
  icon,
  size = "sm",
  tone,
}: {
  children: ReactNode;
  className?: string;
  icon: ReactNode;
  size?: "sm" | "md";
  tone: StatusBadgeTone;
}) {
  return (
    <span
      data-status-badge
      className={cn(
        "inline-flex shrink-0 items-center rounded font-semibold text-white",
        size === "sm"
          ? "h-5 gap-1 px-1.5 text-xs leading-none [&_svg]:h-3 [&_svg]:w-3"
          : "h-6 gap-1.5 px-2 text-xs leading-none [&_svg]:h-3.5 [&_svg]:w-3.5",
        toneClasses[tone],
        className,
      )}
    >
      <span className="inline-flex shrink-0" aria-hidden="true">
        {icon}
      </span>
      <span>{children}</span>
    </span>
  );
}
