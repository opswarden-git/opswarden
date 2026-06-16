import React from "react";
import { IncidentStatus } from "@/lib/queries/incidents";
import { CircleDot, Clock, ShieldAlert, CheckCircle2 } from "lucide-react";
import { useTranslations } from "next-intl";

export function StateChip({ status }: { status: IncidentStatus }) {
  const t = useTranslations("Incidents");

  switch (status) {
    case "open":
      return (
        <span className="border-st-open/20 bg-st-open/10 text-st-open inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize">
          <CircleDot className="h-3 w-3" />
          {t("statusOpen")}
        </span>
      );
    case "acknowledged":
      return (
        <span className="border-st-ack/20 bg-st-ack/10 text-st-ack inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize">
          <Clock className="h-3 w-3" />
          {t("statusAcknowledged")}
        </span>
      );
    case "escalated":
      return (
        <span className="border-st-esc/20 bg-st-esc/10 text-st-esc inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize">
          <ShieldAlert className="h-3 w-3" />
          {t("statusEscalated")}
        </span>
      );
    case "resolved":
      return (
        <span className="border-st-res/20 bg-st-res/10 text-st-res inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-xs font-medium capitalize">
          <CheckCircle2 className="h-3 w-3" />
          {t("statusResolved")}
        </span>
      );
  }
}
