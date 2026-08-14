import React from "react";
import { CircleAlert, Flame, OctagonAlert, TriangleAlert } from "lucide-react";
import { useTranslations } from "next-intl";
import { StatusBadge } from "@/components/ui/StatusBadge";
import type { IncidentSeverity } from "@/lib/queries/incidents";

export function SeverityChip({ severity }: { severity: IncidentSeverity }) {
  const t = useTranslations("Incidents");

  switch (severity) {
    case "low":
      return (
        <StatusBadge tone="neutral" icon={<CircleAlert />}>
          {t("severityLow")}
        </StatusBadge>
      );
    case "medium":
      return (
        <StatusBadge tone="warning" icon={<TriangleAlert />}>
          {t("severityMedium")}
        </StatusBadge>
      );
    case "high":
      return (
        <StatusBadge tone="warning" icon={<OctagonAlert />}>
          {t("severityHigh")}
        </StatusBadge>
      );
    case "critical":
      return (
        <StatusBadge tone="danger" icon={<Flame />}>
          {t("severityCritical")}
        </StatusBadge>
      );
  }
}
