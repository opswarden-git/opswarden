import React from "react";
import type { IncidentStatus } from "@/lib/incident-contract";
import { CircleAlert, Eye, ShieldAlert, CheckCircle2 } from "lucide-react";
import { useTranslations } from "next-intl";
import { StatusBadge } from "@/components/ui/StatusBadge";

export function StateChip({ status }: { status: IncidentStatus }) {
  const t = useTranslations("Incidents");

  switch (status) {
    case "open":
      return (
        <StatusBadge tone="neutral" icon={<CircleAlert />}>
          {t("statusOpen")}
        </StatusBadge>
      );
    case "acknowledged":
      return (
        <StatusBadge tone="info" icon={<Eye />}>
          {t("statusAcknowledged")}
        </StatusBadge>
      );
    case "escalated":
      return (
        <StatusBadge tone="danger" icon={<ShieldAlert />}>
          {t("statusEscalated")}
        </StatusBadge>
      );
    case "resolved":
      return (
        <StatusBadge tone="success" icon={<CheckCircle2 />}>
          {t("statusResolved")}
        </StatusBadge>
      );
  }
}
