import { ChevronRight } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { buttonClassNames } from "@/components/ui/Button";
import { Link } from "@/i18n/routing";
import type { IncidentListItem } from "@/lib/queries/incidents";
import { SeverityChip } from "./SeverityChip";
import { StateChip } from "./StateChip";

function formatAge(createdAt: string, locale: string) {
  const elapsedSeconds = Math.max(
    0,
    Math.floor((Date.now() - new Date(createdAt).getTime()) / 1000),
  );
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  if (elapsedSeconds < 60) return formatter.format(-elapsedSeconds, "second");
  const minutes = Math.floor(elapsedSeconds / 60);
  if (minutes < 60) return formatter.format(-minutes, "minute");
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return formatter.format(-hours, "hour");
  const days = Math.floor(hours / 24);
  if (days < 30) return formatter.format(-days, "day");
  const months = Math.floor(days / 30);
  return formatter.format(-months, "month");
}

export function IncidentRow({ incident }: { incident: IncidentListItem }) {
  const t = useTranslations("Incidents");
  const locale = useLocale();
  const createdAt = new Date(incident.created_at);

  return (
    <tr className="group transition-colors hover:bg-white/[0.04]">
      <td className="px-5 py-3.5 align-middle">
        <StateChip status={incident.status} />
      </td>
      <td className="px-5 py-3.5 align-middle">
        <Link
          href={`/teams/${incident.team_id}/incidents/${incident.id}`}
          className="text-text hover:text-gold font-medium transition-colors"
        >
          {incident.title}
        </Link>
        <div className="text-muted/70 mt-1 font-mono text-xs">
          {t("shortId", { id: incident.id.split("-")[0] })}
        </div>
      </td>
      <td className="px-5 py-3.5 align-middle">
        {incident.assignee ? (
          <div className="flex min-w-0 items-center gap-2">
            <span className="bg-panel-2 text-muted flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold uppercase">
              {incident.assignee.email[0]}
            </span>
            <span className="text-text max-w-56 truncate text-sm" title={incident.assignee.email}>
              {incident.assignee.email}
            </span>
          </div>
        ) : (
          <span className="text-muted text-sm">{t("unassigned")}</span>
        )}
      </td>
      <td className="px-5 py-3.5 align-middle">
        <SeverityChip severity={incident.severity} />
      </td>
      <td
        className="text-muted px-5 py-3.5 align-middle text-sm"
        title={createdAt.toLocaleString(locale)}
      >
        {formatAge(incident.created_at, locale)}
      </td>
      <td className="px-5 py-3.5 text-right align-middle">
        <Link
          href={`/teams/${incident.team_id}/incidents/${incident.id}`}
          className={buttonClassNames({ size: "sm" })}
        >
          {t("openIncident")} <ChevronRight className="h-3.5 w-3.5" />
        </Link>
      </td>
    </tr>
  );
}
