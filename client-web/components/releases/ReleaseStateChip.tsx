import React from "react";
import { CircleDashed, Ban, CheckCircle2, Loader, XCircle } from "lucide-react";
import { useTranslations } from "next-intl";
import type { ReleaseState } from "@/lib/queries/releases";

/** Read-only state pill for a release, mirroring the incident StateChip style. */
export function ReleaseStateChip({ state }: { state: ReleaseState }) {
  const t = useTranslations("Releases");

  const base =
    "inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize";

  switch (state) {
    case "created":
      return (
        <span className={`${base} border-border text-muted bg-white/[0.03]`}>
          <CircleDashed className="h-3 w-3" />
          {t("stateCreated")}
        </span>
      );
    case "in_progress":
      return (
        <span className={`${base} border-st-ack/20 bg-st-ack/10 text-st-ack`}>
          <Loader className="h-3 w-3" />
          {t("stateInProgress")}
        </span>
      );
    case "blocked":
      return (
        <span className={`${base} border-sev-critical/20 bg-sev-critical/10 text-sev-critical`}>
          <Ban className="h-3 w-3" />
          {t("stateBlocked")}
        </span>
      );
    case "completed":
      return (
        <span className={`${base} border-st-res/20 bg-st-res/10 text-st-res`}>
          <CheckCircle2 className="h-3 w-3" />
          {t("stateCompleted")}
        </span>
      );
    case "cancelled":
      return (
        <span className={`${base} border-border text-muted/60 bg-white/[0.03]`}>
          <XCircle className="h-3 w-3" />
          {t("stateCancelled")}
        </span>
      );
  }
}
