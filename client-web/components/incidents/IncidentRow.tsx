import React from "react";
import { Link } from "@/i18n/routing";
import { Incident } from "@/lib/queries/incidents";
import { StateChip } from "./StateChip";
import { SeverityChip } from "./SeverityChip";
import { ChevronRight } from "lucide-react";
import { useTranslations } from "next-intl";

export function IncidentRow({ incident }: { incident: Incident }) {
  const t = useTranslations("Incidents");
  const date = new Date(incident.created_at).toLocaleString();

  return (
    <tr className="group transition-colors hover:bg-white/[0.04]">
      <td className="px-5 py-3.5 align-middle">
        <StateChip status={incident.status} />
      </td>
      <td className="px-5 py-3.5 align-middle">
        <div className="text-text font-medium">{incident.title}</div>
        <div className="text-muted/70 text-opacity-80 mt-1 font-mono text-xs">
          ID: {incident.id.split("-")[0]}...
        </div>
      </td>
      <td className="px-5 py-3.5 align-middle">
        <SeverityChip severity={incident.severity} />
      </td>
      <td className="text-muted px-5 py-3.5 align-middle text-sm">{date}</td>
      <td className="px-5 py-3.5 text-right align-middle">
        <Link
          href={`/incidents/${incident.id}`}
          className="ow-secondary text-muted hover:text-text inline-flex h-8 items-center gap-1.5 rounded-md px-3 text-xs font-medium transition-colors"
        >
          {t("warRoom")} <ChevronRight className="h-3.5 w-3.5" />
        </Link>
      </td>
    </tr>
  );
}
