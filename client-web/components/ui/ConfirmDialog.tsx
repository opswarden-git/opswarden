"use client";

import * as Dialog from "@radix-ui/react-dialog";
import React, { useRef, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "./Button";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  /** Shown on the confirm button while `pending`; defaults to `confirmLabel`. */
  pendingLabel?: string;
  danger?: boolean;
  /** When set, the confirm button stays disabled until the user types this exact
   *  sentinel (e.g. "DELETE") — the dark-pattern guard for destructive actions. */
  requireType?: string;
  requireTypeLabel?: string;
  pending?: boolean;
  error?: string | null;
  /** Optional extra body (e.g. a ban-duration select) rendered under the description. */
  children?: React.ReactNode;
  onConfirm: () => void;
  onClose: () => void;
}

/**
 * Shared confirmation modal for danger / typed-confirm actions across the app
 * (team leave/delete/transfer, incident delete, account delete). Labels are
 * passed in so the dialog stays i18n-namespace-agnostic. The stateful body only
 * mounts while open, so the typed sentinel resets on every open without an effect.
 */
export function ConfirmDialog(props: ConfirmDialogProps) {
  return (
    <Dialog.Root open={props.open} onOpenChange={(open) => !open && props.onClose()}>
      {props.open ? <ConfirmDialogBody {...props} /> : null}
    </Dialog.Root>
  );
}

function ConfirmDialogBody({
  title,
  description,
  confirmLabel,
  cancelLabel,
  pendingLabel,
  danger = false,
  requireType,
  requireTypeLabel,
  pending = false,
  error,
  children,
  onConfirm,
  onClose,
}: ConfirmDialogProps) {
  const [typed, setTyped] = useState("");
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmDisabled = pending || (requireType ? typed !== requireType : false);

  return (
    <Dialog.Portal>
      <Dialog.Overlay className="bg-bg/80 fixed inset-0 z-50 backdrop-blur-sm" />
      <Dialog.Content
        className="surface fixed top-1/2 left-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 space-y-5 rounded-md p-6 shadow-2xl outline-none"
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          cancelRef.current?.focus();
        }}
      >
        <div className="flex gap-3">
          {danger ? (
            <AlertTriangle
              className="text-sev-critical mt-0.5 h-5 w-5 shrink-0"
              aria-hidden="true"
            />
          ) : null}
          <div>
            <Dialog.Title className="text-text text-lg font-semibold">{title}</Dialog.Title>
            <Dialog.Description className="text-muted mt-2 text-sm">
              {description}
            </Dialog.Description>
          </div>
        </div>

        {children}

        {requireType ? (
          <label className="text-text block text-sm font-medium">
            <span>{requireTypeLabel ?? requireType}</span>
            <input
              value={typed}
              onChange={(event) => setTyped(event.target.value)}
              className="ow-input focus-visible:ring-sev-critical/50 mt-2 flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
              autoComplete="off"
              spellCheck={false}
            />
          </label>
        ) : null}

        {error ? (
          <p className="text-sev-critical text-sm" role="alert">
            {error}
          </p>
        ) : null}

        <div className="flex justify-end gap-3 pt-2">
          <Dialog.Close asChild>
            <Button ref={cancelRef} size="lg">
              {cancelLabel}
            </Button>
          </Dialog.Close>
          <Button
            size="lg"
            variant={danger ? "danger" : "primary"}
            onClick={onConfirm}
            disabled={confirmDisabled}
            loading={pending}
          >
            {pending ? (pendingLabel ?? confirmLabel) : confirmLabel}
          </Button>
        </div>
      </Dialog.Content>
    </Dialog.Portal>
  );
}
