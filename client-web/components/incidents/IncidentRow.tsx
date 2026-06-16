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
    <tr className="group transition-colors hover:bg-white/5">
      <td className="px-6 py-4 align-middle">
        <StateChip status={incident.status} />
      </td>
      <td className="px-6 py-4 align-middle">
        <div className="text-text font-medium">{incident.title}</div>
        <div className="text-muted/70 text-opacity-80 mt-1 font-mono text-xs">
          ID: {incident.id.split("-")[0]}...
        </div>
      </td>
      <td className="px-6 py-4 align-middle">
        <SeverityChip severity={incident.severity} />
      </td>
      <td className="px-6 py-4 align-middle text-sm text-muted">{date}</td>
      <td className="px-6 py-4 text-right align-middle">
        <Link
          href={`/incidents/${incident.id}`}
          className="bg-white/5 text-muted hover:text-text border border-white/10 hover:bg-white/10 inline-flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-bold transition-colors"
        >
          {t("warRoom")} <ChevronRight className="h-3 w-3" />
        </Link>
      </td>
    </tr>
  );
}
