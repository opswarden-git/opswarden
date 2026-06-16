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
    <tr className="group border-b border-white/5 transition-colors last:border-0 hover:bg-white/5">
      <td className="p-4 align-middle">
        <StateChip status={incident.status} />
      </td>
      <td className="p-4 align-middle">
        <div className="text-text font-medium">{incident.title}</div>
        <div className="text-muted/70 text-opacity-80 mt-1 font-mono text-xs">
          ID: {incident.id.split("-")[0]}...
        </div>
      </td>
      <td className="p-4 align-middle">
        <SeverityChip severity={incident.severity} />
      </td>
      <td className="p-4 align-middle text-sm text-gray-400">{date}</td>
      <td className="p-4 text-right align-middle">
        <Link
          href={`/incidents/${incident.id}`}
          className="bg-gold/10 text-gold hover:bg-gold hover:text-bg inline-flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-bold tracking-wider transition-colors"
        >
          {t("warRoom")} <ChevronRight className="h-3 w-3" />
        </Link>
      </td>
    </tr>
  );
}
