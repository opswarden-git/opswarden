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
        <span className={`${base} border-border text-rel-created bg-white/[0.03]`}>
          <CircleDashed className="h-3 w-3" />
          {t("stateCreated")}
        </span>
      );
    case "in_progress":
      return (
        <span className={`${base} border-rel-progress/20 bg-rel-progress/10 text-rel-progress`}>
          <Loader className="h-3 w-3" />
          {t("stateInProgress")}
        </span>
      );
    case "blocked":
      return (
        <span className={`${base} border-rel-blocked/20 bg-rel-blocked/10 text-rel-blocked`}>
          <Ban className="h-3 w-3" />
          {t("stateBlocked")}
        </span>
      );
    case "completed":
      return (
        <span className={`${base} border-rel-completed/20 bg-rel-completed/10 text-rel-completed`}>
          <CheckCircle2 className="h-3 w-3" />
          {t("stateCompleted")}
        </span>
      );
    case "cancelled":
      return (
        <span className={`${base} border-border text-rel-cancelled bg-white/[0.03]`}>
          <XCircle className="h-3 w-3" />
          {t("stateCancelled")}
        </span>
      );
  }
}
