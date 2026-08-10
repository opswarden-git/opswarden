"use client";

import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";

export function RailToggle({
  className,
  direction,
  label,
  onClick,
}: {
  className?: string;
  direction: "left" | "right";
  label: string;
  onClick: () => void;
}) {
  const Icon = direction === "left" ? ChevronLeft : ChevronRight;

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "border-border bg-panel text-muted hover:bg-panel-2 hover:text-text focus-visible:ring-gold/50 absolute z-30 flex h-8 w-4 items-center justify-center rounded-full border opacity-55 shadow-sm transition-[color,background-color,opacity] hover:opacity-100 focus-visible:ring-2 focus-visible:outline-none",
        className,
      )}
    >
      <Icon className="h-3 w-3" strokeWidth={1.75} aria-hidden="true" />
    </button>
  );
}
