import React from "react";
import { IncidentSeverity } from "@/lib/queries/incidents";
import { AlertCircle, AlertTriangle, AlertOctagon, Flame } from "lucide-react";
import { useTranslations } from "next-intl";

export function SeverityChip({ severity }: { severity: IncidentSeverity }) {
  const t = useTranslations("Incidents");

  switch (severity) {
    case "low":
      return (
        <span className="text-sev-low inline-flex items-center gap-1 text-xs font-medium capitalize">
          <AlertCircle className="h-3.5 w-3.5" />
          {t("severityLow")}
        </span>
      );
    case "medium":
      return (
        <span className="text-sev-medium inline-flex items-center gap-1 text-xs font-medium capitalize">
          <AlertTriangle className="h-3.5 w-3.5" />
          {t("severityMedium")}
        </span>
      );
    case "high":
      return (
        <span className="text-sev-high inline-flex items-center gap-1 text-xs font-medium capitalize">
          <AlertOctagon className="h-3.5 w-3.5" />
          {t("severityHigh")}
        </span>
      );
    case "critical":
      return (
        <span className="text-sev-critical inline-flex items-center gap-1 text-xs font-bold uppercase">
          <Flame className="ow-decorative-motion h-3.5 w-3.5 animate-pulse" />
          {t("severityCritical")}
        </span>
      );
  }
}
