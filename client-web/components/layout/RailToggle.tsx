"use client";

import { cn } from "@/lib/utils";

/**
 * The separator between two rails, doubling as its own toggle — the VS Code
 * sash grammar: an invisible 4px strip whose `::before` fills in on hover.
 *
 * VS Code delays that fill by 300ms so a cursor merely crossing the seam does
 * not make it blink; the delay here is applied on hover-in only, so releasing
 * is immediate. Unlike VS Code the strip collapses on click rather than
 * resizing: this product has no rail sizes to remember.
 */
export function RailToggle({
  className,
  label,
  onClick,
  side,
}: {
  className?: string;
  label: string;
  onClick: () => void;
  side: "left" | "right";
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "group absolute inset-y-0 z-30 w-1 cursor-pointer bg-transparent",
        "focus-visible:ring-gold/50 focus-visible:ring-2 focus-visible:outline-none",
        side === "left" ? "left-0" : "right-0",
        className,
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "block h-full w-full bg-transparent transition-colors duration-100",
          "group-hover:bg-gold group-hover:delay-300",
          "group-active:bg-gold group-active:delay-0",
          "group-focus-visible:bg-gold group-focus-visible:delay-0",
        )}
      />
    </button>
  );
}
