import React from "react";
import { IncidentStatus } from "@/lib/queries/incidents";
import { CircleDot, Clock, ShieldAlert, CheckCircle2 } from "lucide-react";
import { useTranslations } from "next-intl";

export function StateChip({ status }: { status: IncidentStatus }) {
  const t = useTranslations("Incidents");

  switch (status) {
    case "open":
      return (
        <span className="inline-flex items-center gap-1.5 rounded-full border border-red-500/20 bg-red-500/10 px-2 py-1 text-xs font-medium text-red-400 capitalize">
          <CircleDot className="h-3 w-3" />
          {t("statusOpen")}
        </span>
      );
    case "acknowledged":
      return (
        <span className="bg-gold/10 border-gold/20 text-gold inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize">
          <Clock className="h-3 w-3" />
          {t("statusAcknowledged")}
        </span>
      );
    case "escalated":
      return (
        <span className="inline-flex items-center gap-1.5 rounded-full border border-purple-500/20 bg-purple-500/10 px-2 py-1 text-xs font-medium text-purple-400 capitalize">
          <ShieldAlert className="h-3 w-3" />
          {t("statusEscalated")}
        </span>
      );
    case "resolved":
      return (
        <span className="inline-flex items-center gap-1.5 rounded-full border border-green-500/20 bg-green-500/10 px-2 py-1 text-xs font-medium text-green-400 capitalize">
          <CheckCircle2 className="h-3 w-3" />
          {t("statusResolved")}
        </span>
      );
  }
}
