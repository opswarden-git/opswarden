import React from "react";
import { CircleDashed, Ban, CheckCircle2, Loader, XCircle } from "lucide-react";
import { useTranslations } from "next-intl";
import { StatusBadge } from "@/components/ui/StatusBadge";
import type { ReleaseState } from "@/lib/release-contract";

/** Read-only state panel for a release, mirroring the incident StateChip style. */
export function ReleaseStateChip({ state }: { state: ReleaseState }) {
  const t = useTranslations("Releases");

  switch (state) {
    case "created":
      return (
        <StatusBadge tone="neutral" icon={<CircleDashed />}>
          {t("stateCreated")}
        </StatusBadge>
      );
    case "in_progress":
      return (
        <StatusBadge tone="info" icon={<Loader />}>
          {t("stateInProgress")}
        </StatusBadge>
      );
    case "blocked":
      return (
        <StatusBadge tone="danger" icon={<Ban />}>
          {t("stateBlocked")}
        </StatusBadge>
      );
    case "completed":
      return (
        <StatusBadge tone="success" icon={<CheckCircle2 />}>
          {t("stateCompleted")}
        </StatusBadge>
      );
    case "cancelled":
      return (
        <StatusBadge tone="neutral" icon={<XCircle />}>
          {t("stateCancelled")}
        </StatusBadge>
      );
  }
}
