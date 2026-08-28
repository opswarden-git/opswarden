"use client";

import { ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * A collapsible pane section, following the VS Code Explorer grammar: a short
 * uppercase header whose *whole row* is the click target, and a twisty that is
 * hidden — not disabled — when the section cannot collapse.
 */
export function PaneSection({
  children,
  className,
  collapsible = true,
  defaultOpen = true,
  title,
  titleId,
}: {
  children: ReactNode;
  className?: string;
  collapsible?: boolean;
  defaultOpen?: boolean;
  title: ReactNode;
  /** Lets a caller keep an `aria-labelledby` relationship on its own wrapper. */
  titleId?: string;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const expanded = collapsible ? open : true;
  const headerClasses =
    "text-muted-2 flex h-6 w-full items-center gap-1 pr-2 text-[11px] font-semibold tracking-wider uppercase";

  return (
    <section className={cn("min-w-0", className)} aria-labelledby={titleId}>
      {collapsible ? (
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen((current) => !current)}
          className={cn(
            headerClasses,
            "hover:text-text focus-visible:ring-gold/50 cursor-pointer rounded-sm pl-1 transition-colors focus-visible:ring-2 focus-visible:outline-none",
          )}
        >
          <ChevronRight
            className={cn("h-3.5 w-3.5 shrink-0 transition-transform", open && "rotate-90")}
            aria-hidden="true"
          />
          <span id={titleId} className="min-w-0 truncate">
            {title}
          </span>
        </button>
      ) : (
        <div className={cn(headerClasses, "cursor-default pl-3")}>
          <span className="min-w-0 truncate">{title}</span>
        </div>
      )}

      {/* Serré sous son titre, aéré avant la section suivante : le contenu doit
          appartenir visuellement à son en-tête, pas au voisin d'en dessous. */}
      {expanded ? <div className="pt-1 pb-4">{children}</div> : null}
    </section>
  );
}
