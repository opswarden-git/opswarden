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
        "text-muted hover:text-text focus-visible:ring-gold/50 absolute z-30 flex h-10 w-4 items-center justify-center opacity-40 transition-[color,opacity] hover:opacity-100 focus-visible:ring-2 focus-visible:outline-none",
        className,
      )}
    >
      <Icon className="h-5 w-5" strokeWidth={1.8} aria-hidden="true" />
    </button>
  );
}
