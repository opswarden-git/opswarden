"use client";

import * as RadixDialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { Children, type ReactElement, type ReactNode, type RefObject } from "react";
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
  const bodyChildren = Children.toArray(children);

  return (
    <RadixDialog.Root open={open} onOpenChange={onOpenChange}>
      {trigger ? <RadixDialog.Trigger asChild>{trigger}</RadixDialog.Trigger> : null}

      <RadixDialog.Portal>
        <RadixDialog.Overlay className="bg-bg/80 data-[state=closed]:animate-dialog-fade-out data-[state=open]:animate-dialog-fade-in fixed inset-0 z-50" />
        <RadixDialog.Content
          data-dialog-part="content"
          {...(description ? {} : { "aria-describedby": undefined })}
          className={cn(
            "surface elevated fixed z-50 flex max-h-[calc(100dvh-2rem)] flex-col overflow-hidden outline-none data-[state=closed]:pointer-events-none",
            // Both variants are the same surface anchored to the bottom edge; a
            // modal simply lifts off it and centres from `sm` up. A phone has no
            // room for a floating card, and a sheet is where a thumb already is.
            "data-[state=closed]:animate-sheet-content-hide data-[state=open]:animate-sheet-content-show right-0 bottom-0 left-0 mt-auto w-full rounded-t-[var(--ow-radius-lg)] rounded-b-none",
            variant === "modal" &&
              "sm:data-[state=closed]:animate-dialog-fade-out sm:data-[state=open]:animate-dialog-fade-in sm:inset-x-auto sm:top-1/2 sm:bottom-auto sm:left-1/2 sm:mt-0 sm:w-[calc(100%-2rem)] sm:-translate-x-1/2 sm:-translate-y-1/2 sm:rounded-[var(--ow-radius-lg)]",
            variant === "modal" && sizeClasses[size],
            contentClassName,
          )}
          onOpenAutoFocus={(event) => {
            if (!initialFocus?.current) return;
            event.preventDefault();
            initialFocus.current.focus();
          }}
        >
          <div
            className={cn(
              "bg-border mx-auto mt-3 h-1.5 w-12 shrink-0 rounded-full",
              // The grip means "this came from the bottom edge"; a centred
              // dialog did not.
              variant === "modal" && "sm:hidden",
            )}
            aria-hidden="true"
          />
          <header
            className={cn(
              "border-border/60 relative flex shrink-0 items-center justify-between border-b px-4 py-3",
              closeLabel && "pr-12",
              titleHidden && "sr-only",
            )}
          >
            <div className="flex min-w-0 items-center gap-2">
              {icon}
              <div className="min-w-0">
                <RadixDialog.Title className="text-text text-sm leading-5 font-semibold">
                  {title}
                </RadixDialog.Title>
                {description ? (
                  <RadixDialog.Description className="text-muted mt-0.5 text-xs leading-4">
                    {description}
                  </RadixDialog.Description>
                ) : null}
              </div>
            </div>
            {closeLabel ? (
              <RadixDialog.Close asChild>
                <IconButton
                  className="absolute top-2.5 right-3"
                  label={closeLabel}
                  size="sm"
                  variant="ghost"
                >
                  <X className="h-4 w-4" aria-hidden="true" />
                </IconButton>
              </RadixDialog.Close>
            ) : null}
          </header>

          {/* A confirmation whose whole content is its description has nothing
              to put here, and an empty body still drew its padding and the rule
              above the footer — a band of nothing between two lines. */}
          {bodyChildren.length > 0 ? (
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
          ) : null}

          {footer ? (
            <footer
              data-dialog-part="footer"
              className="border-border/60 bg-panel/30 flex shrink-0 items-center justify-end gap-2 border-t px-4 py-2"
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
