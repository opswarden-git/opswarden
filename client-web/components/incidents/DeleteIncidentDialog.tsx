"use client";

import React, { useState } from "react";
import { AlertTriangle } from "lucide-react";
import { useTranslations } from "next-intl";

interface DeleteIncidentDialogProps {
  open: boolean;
  /** Incident title, surfaced in the confirmation message. */
  title: string;
  pending?: boolean;
  error?: string | null;
  onConfirm: () => void;
  onClose: () => void;
}

/**
 * Typed-confirmation modal for deleting an incident. The body only mounts while
 * open, so the typed sentinel resets on every open without a reset effect.
 */
export function DeleteIncidentDialog(props: DeleteIncidentDialogProps) {
  if (!props.open) return null;
  return <DeleteIncidentDialogBody {...props} />;
}

function DeleteIncidentDialogBody({
  title,
  pending = false,
  error,
  onConfirm,
  onClose,
}: DeleteIncidentDialogProps) {
  const t = useTranslations("Incidents");
  const [typed, setTyped] = useState("");
  const confirmDisabled = pending || typed !== "DELETE";

  return (
    <div className="bg-bg/80 fixed inset-0 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
      <div className="surface w-full max-w-md space-y-5 rounded-md p-6 shadow-2xl">
        <div className="flex gap-3">
          <AlertTriangle className="text-sev-critical mt-0.5 h-5 w-5 shrink-0" />
          <div>
            <h2 className="text-text text-lg font-semibold">{t("deleteIncident")}</h2>
            <p className="text-muted mt-2 text-sm">{t("deleteIncidentConfirm", { title })}</p>
          </div>
        </div>

        <input
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
          className="ow-input focus-visible:ring-sev-critical/50 flex h-10 w-full rounded-md px-3 py-2 text-sm transition-colors"
          placeholder="DELETE"
        />

        {error ? <p className="text-sm text-red-400">{error}</p> : null}

        <div className="flex justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="ow-secondary h-10 rounded-md px-4 text-sm font-medium transition-colors"
          >
            {t("cancel")}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={confirmDisabled}
            className="ow-danger inline-flex h-10 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-colors disabled:opacity-50"
          >
            {pending ? t("processing") : t("deleteIncident")}
          </button>
        </div>
      </div>
    </div>
  );
}
