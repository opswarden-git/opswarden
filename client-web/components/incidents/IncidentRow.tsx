import { useLocale, useTranslations } from "next-intl";
import {
  OperationalTableCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "@/components/ui/OperationalTable";
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

function incidentHref(incident: IncidentListItem) {
  return `/teams/${incident.team_id}/incidents/${incident.id}`;
}

function IncidentIdentity({ incident }: { incident: IncidentListItem }) {
  const t = useTranslations("Incidents");
  return (
    <>
      <Link
        href={incidentHref(incident)}
        className="text-text hover:text-gold font-medium transition-colors"
      >
        {incident.title}
      </Link>
      <div className="text-muted-2 mt-1 font-mono text-xs">
        {t("shortId", { id: incident.id.split("-")[0] })}
      </div>
    </>
  );
}

function IncidentAssignee({ incident }: { incident: IncidentListItem }) {
  const t = useTranslations("Incidents");
  if (!incident.assignee) return <span className="text-muted text-sm">{t("unassigned")}</span>;

  return (
    <div className="flex min-w-0 items-center gap-2">
      <span className="bg-panel-2 text-muted flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold uppercase">
        {incident.assignee.email[0]}
      </span>
      <span className="text-text min-w-0 truncate text-sm" title={incident.assignee.email}>
        {incident.assignee.email}
      </span>
    </div>
  );
}

export function IncidentRow({ incident }: { incident: IncidentListItem }) {
  const locale = useLocale();
  const createdAt = new Date(incident.created_at);

  return (
    <OperationalTableRow>
      <OperationalTableCell>
        <StateChip status={incident.status} />
      </OperationalTableCell>
      <OperationalTableRowHeader className="w-[34%]">
        <IncidentIdentity incident={incident} />
      </OperationalTableRowHeader>
      <OperationalTableCell className="max-w-64">
        <IncidentAssignee incident={incident} />
      </OperationalTableCell>
      <OperationalTableCell>
        <SeverityChip severity={incident.severity} />
      </OperationalTableCell>
      <OperationalTableCell className="text-muted text-sm">
        <time dateTime={incident.created_at} title={createdAt.toLocaleString(locale)}>
          {formatAge(incident.created_at, locale)}
        </time>
      </OperationalTableCell>
    </OperationalTableRow>
  );
}

export function IncidentMobileRecord({ incident }: { incident: IncidentListItem }) {
  const t = useTranslations("Incidents");
  const locale = useLocale();
  const createdAt = new Date(incident.created_at);

  return (
    <li className="px-4 py-4">
      <div data-incident-field="identity">
        <IncidentIdentity incident={incident} />
      </div>
      <div data-incident-field="state" className="mt-3 flex flex-wrap items-center gap-2">
        <StateChip status={incident.status} />
        <SeverityChip severity={incident.severity} />
      </div>
      <div
        data-incident-field="assignee"
        className="mt-3 grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2"
      >
        <span className="text-muted text-xs">{t("colAssignee")}</span>
        <IncidentAssignee incident={incident} />
      </div>
      <div
        data-incident-field="age"
        className="mt-2 grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2"
      >
        <span className="text-muted text-xs">{t("colAge")}</span>
        <time
          className="text-muted text-sm"
          dateTime={incident.created_at}
          title={createdAt.toLocaleString(locale)}
        >
          {formatAge(incident.created_at, locale)}
        </time>
      </div>
    </li>
  );
}
