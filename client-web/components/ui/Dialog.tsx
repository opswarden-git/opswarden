"use client";

import * as RadixDialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import type { ReactElement, ReactNode, RefObject } from "react";
import { cn } from "@/lib/utils";
import { IconButton } from "./Button";

type DialogSize = "sm" | "md" | "lg";

const sizeClasses: Record<DialogSize, string> = {
  sm: "max-w-md",
  md: "max-w-lg",
  lg: "max-w-2xl",
};

interface DialogProps {
  bodyClassName?: string;
  children: ReactNode;
  /** Renders the header close button when provided; the visible footer may be the only close action. */
  closeLabel?: string;
  contentClassName?: string;
  description: ReactNode;
  footer?: ReactNode;
  icon?: ReactNode;
  initialFocus?: RefObject<HTMLElement | null>;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  size?: DialogSize;
  title: ReactNode;
  trigger?: ReactElement;
  variant?: "modal" | "sheet";
}

/**
 * Shared modal shell. Radix owns modal semantics, focus containment, Escape and
 * focus restoration; feature dialogs only provide content and state.
 */
export function Dialog({
  bodyClassName,
  children,
  closeLabel,
  contentClassName,
  description,
  footer,
  icon,
  initialFocus,
  onOpenChange,
  open,
  size = "md",
  title,
  trigger,
  variant = "modal",
}: DialogProps) {
  return (
    <RadixDialog.Root open={open} onOpenChange={onOpenChange}>
      {trigger ? <RadixDialog.Trigger asChild>{trigger}</RadixDialog.Trigger> : null}

      <RadixDialog.Portal>
        <RadixDialog.Overlay className="bg-bg/80 data-[state=closed]:animate-dialog-overlay-hide data-[state=open]:animate-dialog-overlay-show fixed inset-0 z-50 backdrop-blur-sm" />
        <RadixDialog.Content
          data-dialog-part="content"
          className={cn(
            "surface fixed z-50 flex max-h-[calc(100dvh-2rem)] flex-col overflow-hidden shadow-2xl outline-none",
            variant === "modal"
              ? "data-[state=closed]:animate-dialog-content-hide data-[state=open]:animate-dialog-content-show top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[calc(100%-2rem)] rounded-md"
              : "data-[state=closed]:animate-sheet-content-hide data-[state=open]:animate-sheet-content-show bottom-0 left-0 right-0 mt-auto w-full rounded-t-2xl",
            variant === "modal" && sizeClasses[size],
            contentClassName,
          )}
          onOpenAutoFocus={(event) => {
            if (!initialFocus?.current) return;
            event.preventDefault();
            initialFocus.current.focus();
          }}
        >
          {variant === "sheet" ? (
            <div className="bg-border mx-auto mt-3 h-1.5 w-12 shrink-0 rounded-full" aria-hidden="true" />
          ) : null}
          <header className="border-border relative flex shrink-0 items-start gap-3 border-b p-6 pr-14">
            {icon}
            <div className="min-w-0">
              <RadixDialog.Title className="text-text text-lg font-semibold">
                {title}
              </RadixDialog.Title>
              <RadixDialog.Description className="text-muted mt-1 text-sm">
                {description}
              </RadixDialog.Description>
            </div>
            {closeLabel ? (
              <RadixDialog.Close asChild>
                <IconButton
                  className="absolute top-4 right-4"
                  label={closeLabel}
                  size="sm"
                  variant="ghost"
                >
                  <X className="h-4 w-4" aria-hidden="true" />
                </IconButton>
              </RadixDialog.Close>
            ) : null}
          </header>

          <div
            data-dialog-part="body"
            className={cn("min-h-0 flex-1 overflow-y-auto p-6", bodyClassName)}
          >
            {children}
          </div>

          {footer ? (
            <footer
              data-dialog-part="footer"
              className="border-border flex shrink-0 justify-end gap-2 border-t px-6 py-4"
            >
              {footer}
            </footer>
          ) : null}
        </RadixDialog.Content>
      </RadixDialog.Portal>
    </RadixDialog.Root>
  );
}

/** Close action for a button rendered inside the shared Dialog shell. */
export function DialogClose({ children }: { children: ReactElement }) {
  return <RadixDialog.Close asChild>{children}</RadixDialog.Close>;
}
