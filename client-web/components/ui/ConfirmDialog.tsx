"use client";

import React, { useState } from "react";
import { AlertTriangle } from "lucide-react";

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
  pending?: boolean;
  error?: string | null;
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
  if (!props.open) return null;
  return <ConfirmDialogBody {...props} />;
}

function ConfirmDialogBody({
  title,
  description,
  confirmLabel,
  cancelLabel,
  pendingLabel,
  danger = false,
  requireType,
  pending = false,
  error,
  onConfirm,
  onClose,
}: ConfirmDialogProps) {
  const [typed, setTyped] = useState("");
  const confirmDisabled = pending || (requireType ? typed !== requireType : false);

  return (
    <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
      <div className="surface w-full max-w-md space-y-5 rounded-md p-6 shadow-2xl">
        <div className="flex gap-3">
          {danger ? <AlertTriangle className="text-sev-critical mt-0.5 h-5 w-5 shrink-0" /> : null}
          <div>
            <h2 className="text-text text-lg font-semibold">{title}</h2>
            <p className="text-muted mt-2 text-sm">{description}</p>
          </div>
        </div>

        {requireType ? (
          <input
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            className="ow-input focus-visible:ring-sev-critical/50 flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
            placeholder={requireType}
          />
        ) : null}

        {error ? <p className="text-sm text-red-400">{error}</p> : null}

        <div className="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="ow-secondary h-10 rounded-md px-4 text-sm font-medium transition-colors"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={confirmDisabled}
            className={`inline-flex h-10 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50 ${
              danger ? "ow-danger" : "ow-primary"
            }`}
          >
            {pending ? (pendingLabel ?? confirmLabel) : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
