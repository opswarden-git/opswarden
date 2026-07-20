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
}: DialogProps) {
  return (
    <RadixDialog.Root open={open} onOpenChange={onOpenChange}>
      {trigger ? <RadixDialog.Trigger asChild>{trigger}</RadixDialog.Trigger> : null}

      <RadixDialog.Portal>
        <RadixDialog.Overlay className="bg-bg/80 fixed inset-0 z-50 backdrop-blur-sm" />
        <RadixDialog.Content
          data-dialog-part="content"
          className={cn(
            "surface fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-md shadow-2xl outline-none",
            sizeClasses[size],
            contentClassName,
          )}
          onOpenAutoFocus={(event) => {
            if (!initialFocus?.current) return;
            event.preventDefault();
            initialFocus.current.focus();
          }}
        >
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
