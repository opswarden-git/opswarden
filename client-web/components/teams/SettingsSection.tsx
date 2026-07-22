"use client";

import { ChevronDown, ChevronRight } from "lucide-react";
import React, { useId, useState } from "react";
import { cn } from "@/lib/utils";

export interface SettingsSectionProps {
  badge?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  collapsible?: boolean;
  defaultOpen?: boolean;
  description?: React.ReactNode;
  hasActiveError?: boolean;
  isPending?: boolean;
  title: React.ReactNode;
}

export function SettingsSection({
  badge,
  children,
  className,
  collapsible = false,
  defaultOpen = false,
  description,
  hasActiveError = false,
  isPending = false,
  title,
}: SettingsSectionProps) {
  const [internalOpen, setInternalOpen] = useState(defaultOpen);
  const contentId = useId();

  // If there's an active error or pending operation inside, force section to remain open
  const isOpen = collapsible ? internalOpen || hasActiveError || isPending : true;

  const toggle = () => {
    if (!collapsible) return;
    setInternalOpen((prev) => !prev);
  };

  return (
    <section className={cn("surface rounded-md", className)}>
      {collapsible ? (
        <button
          type="button"
          aria-expanded={isOpen}
          aria-controls={contentId}
          onClick={toggle}
          className="border-border hover:bg-panel-2/50 flex w-full items-start justify-between gap-4 border-b px-6 py-4 text-left transition-colors"
        >
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h2 className="text-text font-semibold">{title}</h2>
              {badge}
            </div>
            {description ? <p className="text-muted mt-1 text-sm">{description}</p> : null}
          </div>
          <span className="text-muted mt-0.5 flex shrink-0 items-center gap-1 text-sm font-medium">
            {isOpen ? (
              <ChevronDown className="h-5 w-5" aria-hidden="true" />
            ) : (
              <ChevronRight className="h-5 w-5" aria-hidden="true" />
            )}
          </span>
        </button>
      ) : (
        <div className="border-border border-b px-6 py-4">
          <div className="flex items-center gap-2">
            <h2 className="text-text font-semibold">{title}</h2>
            {badge}
          </div>
          {description ? <p className="text-muted mt-1 text-sm">{description}</p> : null}
        </div>
      )}

      {isOpen ? (
        <div id={contentId} className="p-6">
          {children}
        </div>
      ) : null}
    </section>
  );
}
