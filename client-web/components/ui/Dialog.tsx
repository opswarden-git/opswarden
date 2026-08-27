"use client";

import * as RadixDialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import type { ReactElement, ReactNode, RefObject } from "react";
import { cn } from "@/lib/utils";
import { IconButton } from "./Button";

type DialogSize = "sm" | "md" | "lg";

/**
 * Three widths, chosen by how much a form actually needs rather than by how
 * much room is available. A dialog wide enough to hold anything invites being
 * filled: the previous smallest was 448px, which is more than a single field
 * and a sentence ever require.
 */
const sizeClasses: Record<DialogSize, string> = {
  sm: "max-w-sm", // 384px — one field, or one question
  md: "max-w-lg", // 512px — a handful of fields
  lg: "max-w-[40rem]", // 640px — grouped forms with side-by-side fields
};

interface DialogProps {
  bodyClassName?: string;
  children: ReactNode;
  /** Renders the header close button when provided; the visible footer may be the only close action. */
  closeLabel?: string;
  contentClassName?: string;
  /** Optional. Omit rather than restating the title in other words. */
  description?: ReactNode;
  footer?: ReactNode;
  icon?: ReactNode;
  initialFocus?: RefObject<HTMLElement | null>;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  size?: DialogSize;
  title: ReactNode;
  /** Keeps the accessible name while removing the visible heading. */
  titleHidden?: boolean;
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
  titleHidden = false,
  trigger,
  variant = "modal",
}: DialogProps) {
  return (
    <RadixDialog.Root open={open} onOpenChange={onOpenChange}>
      {trigger ? <RadixDialog.Trigger asChild>{trigger}</RadixDialog.Trigger> : null}

      <RadixDialog.Portal>
        <RadixDialog.Overlay className="bg-bg/80 fixed inset-0 z-50" />
        <RadixDialog.Content
          data-dialog-part="content"
          {...(description ? {} : { "aria-describedby": undefined })}
          className={cn(
            "surface fixed z-50 flex max-h-[calc(100dvh-2rem)] flex-col overflow-hidden shadow-2xl outline-none",
            variant === "modal"
              ? "surface-floating inset-x-4 top-4 bottom-4 w-auto sm:inset-x-auto sm:top-1/2 sm:bottom-auto sm:left-1/2 sm:w-[calc(100%-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2"
              : "surface-floating-top data-[state=closed]:animate-sheet-content-hide data-[state=open]:animate-sheet-content-show right-0 bottom-0 left-0 mt-auto w-full",
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
            <div
              className="bg-border mx-auto mt-3 h-1.5 w-12 shrink-0 rounded-full"
              aria-hidden="true"
            />
          ) : null}
          <header
            className={cn(
              "relative flex shrink-0 items-start gap-3 px-4 pt-4",
              closeLabel && "pr-14",
              titleHidden && "sr-only",
            )}
          >
            {icon}
            <div className="min-w-0">
              <RadixDialog.Title className="text-text text-lg font-semibold">
                {title}
              </RadixDialog.Title>
              {description ? (
                <RadixDialog.Description className="text-muted mt-1 text-sm">
                  {description}
                </RadixDialog.Description>
              ) : null}
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
            className={cn(
              "min-h-0 flex-1 overflow-y-auto p-4",
              // The rule below the body appears only when there is a footer to
              // separate it from, and only while the body can actually scroll.
              footer && "scroll-divider",
              bodyClassName,
            )}
          >
            {children}
          </div>

          {footer ? (
            <footer data-dialog-part="footer" className="flex shrink-0 justify-end gap-2 px-4 pb-4">
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
