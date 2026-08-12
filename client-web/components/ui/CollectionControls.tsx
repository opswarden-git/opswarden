"use client";

import { ChevronDown, ChevronUp } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Button } from "./Button";
import { Dialog, DialogClose } from "./Dialog";
import { cn } from "@/lib/utils";

export type CollectionFilterOption = {
  label: string;
  value: string;
};

export function TableFilterControl({
  activeLabel,
  className,
  label,
  onChange,
  options,
  value,
}: {
  activeLabel?: string;
  className?: string;
  label: string;
  onChange: (value: string) => void;
  options: CollectionFilterOption[];
  value: string;
}) {
  return (
    <label
      className={cn(
        "text-muted hover:text-text focus-within:ring-gold/50 relative inline-flex cursor-pointer items-center gap-1 rounded-sm uppercase transition-colors focus-within:ring-2 focus-within:outline-none",
        className,
      )}
      title={activeLabel ? `${label}: ${activeLabel}` : label}
    >
      <span>{label}</span>
      <ChevronDown className="h-3 w-3" aria-hidden="true" />
      <select
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="absolute inset-0 cursor-pointer opacity-0"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

export function TableSortControl({
  direction,
  label,
  onToggle,
}: {
  direction?: "ascending" | "descending";
  label: string;
  onToggle: () => void;
}) {
  const Icon = direction === "ascending" ? ChevronUp : ChevronDown;

  return (
    <button
      type="button"
      onClick={onToggle}
      className="text-muted hover:text-text focus-visible:ring-gold/50 inline-flex items-center gap-1 rounded-sm uppercase transition-colors focus-visible:ring-2 focus-visible:outline-none"
    >
      <span>{label}</span>
      <Icon className="h-3 w-3" aria-hidden="true" />
    </button>
  );
}

export function MobileCollectionFilters({
  activeCount,
  children,
  clearLabel,
  closeLabel,
  description,
  doneLabel,
  label,
  onClear,
  title,
}: {
  activeCount: number;
  children: ReactNode;
  clearLabel: string;
  closeLabel: string;
  description: string;
  doneLabel: string;
  label: string;
  onClear: () => void;
  title: string;
}) {
  const [open, setOpen] = useState(false);
  const triggerLabel = activeCount > 0 ? `${label} (${activeCount})` : label;

  return (
    <Dialog
      open={open}
      onOpenChange={setOpen}
      variant="sheet"
      title={title}
      description={description}
      closeLabel={closeLabel}
      trigger={
        <Button size="sm" className="uppercase lg:hidden">
          {triggerLabel}
        </Button>
      }
      footer={
        <>
          {activeCount > 0 ? (
            <Button size="sm" variant="ghost" onClick={onClear}>
              {clearLabel}
            </Button>
          ) : null}
          <DialogClose>
            <Button size="sm" variant="primary">
              {doneLabel}
            </Button>
          </DialogClose>
        </>
      }
    >
      <div className="space-y-4">{children}</div>
    </Dialog>
  );
}
