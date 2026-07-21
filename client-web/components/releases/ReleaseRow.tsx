import { useLocale, useTranslations } from "next-intl";
import {
  OperationalTableCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "@/components/ui/OperationalTable";
import { Link } from "@/i18n/routing";
import type { ReleaseListItem } from "@/lib/queries/releases";
import { ReleaseStateChip } from "./ReleaseStateChip";

export function formatAge(createdAt: string, locale: string) {
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
  return formatter.format(-Math.floor(days / 30), "month");
}

function ReleaseIdentity({
  release,
  hrefFor,
}: {
  release: ReleaseListItem;
  hrefFor: (id: string) => string;
}) {
  return (
    <Link
      href={hrefFor(release.release_id)}
      className="text-text hover:text-gold font-medium transition-colors"
    >
      {release.title}
    </Link>
  );
}

function ReleaseProgress({ release }: { release: ReleaseListItem }) {
  const t = useTranslations("Releases");
  const percentage =
    release.progress.total === 0
      ? 0
      : Math.round((release.progress.completed / release.progress.total) * 100);

  return (
    <div className="flex min-w-32 items-center gap-3">
      <div
        role="progressbar"
        aria-label={t("progressLabel", { title: release.title })}
        aria-valuemin={0}
        aria-valuemax={release.progress.total}
        aria-valuenow={release.progress.completed}
        className="bg-panel-2 h-1.5 min-w-20 flex-1 overflow-hidden rounded-full"
      >
        <span
          className="bg-gold block h-full rounded-full"
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-muted text-xs tabular-nums">
        {t("progressCount", release.progress)}
      </span>
    </div>
  );
}

function ReleaseBlockers({ release }: { release: ReleaseListItem }) {
  const t = useTranslations("Releases");
  if (release.blockers.length === 0) {
    return <span className="text-muted">—</span>;
  }
  return (
    <ul className="space-y-1" aria-label={t("colBlockers")}>
      {release.blockers.map((blocker) => (
        <li key={blocker.incident_id}>
          <Link
            href={`/teams/${release.team_id}/incidents/${blocker.incident_id}`}
            className="text-sev-critical hover:text-sev-high block truncate transition-colors"
            title={blocker.title}
          >
            {blocker.title}
          </Link>
        </li>
      ))}
    </ul>
  );
}

export function ReleaseRow({
  release,
  hrefFor,
}: {
  release: ReleaseListItem;
  hrefFor: (id: string) => string;
}) {
  const locale = useLocale();
  const createdAt = new Date(release.created_at);
  const t = useTranslations("Releases");

  return (
    <OperationalTableRow>
      <OperationalTableRowHeader className="w-[25%]">
        <ReleaseIdentity release={release} hrefFor={hrefFor} />
      </OperationalTableRowHeader>
      <OperationalTableCell>
        <ReleaseStateChip state={release.state} />
      </OperationalTableCell>
      <OperationalTableCell>
        <ReleaseProgress release={release} />
      </OperationalTableCell>
      <OperationalTableCell className="text-text max-w-56">
        {release.next_step?.name ?? <span className="text-muted">{t("noNextStep")}</span>}
      </OperationalTableCell>
      <OperationalTableCell className="text-muted whitespace-nowrap">
        <time dateTime={release.created_at} title={createdAt.toLocaleString(locale)}>
          {formatAge(release.created_at, locale)}
        </time>
      </OperationalTableCell>
      <OperationalTableCell className="max-w-72">
        <ReleaseBlockers release={release} />
      </OperationalTableCell>
    </OperationalTableRow>
  );
}

export function ReleaseMobileRecord({
  release,
  hrefFor,
}: {
  release: ReleaseListItem;
  hrefFor: (id: string) => string;
}) {
  const t = useTranslations("Releases");
  const locale = useLocale();
  const createdAt = new Date(release.created_at);

  return (
    <li className="px-4 py-4">
      <div data-release-field="identity">
        <ReleaseIdentity release={release} hrefFor={hrefFor} />
      </div>
      <div data-release-field="state" className="mt-3 flex flex-wrap items-center gap-2">
        <ReleaseStateChip state={release.state} />
      </div>
      {release.blockers.length > 0 && (
        <div data-release-field="blockers" className="mt-3">
          <span className="text-muted text-xs block mb-1">{t("colBlockers")}</span>
          <ReleaseBlockers release={release} />
        </div>
      )}
      <div
        data-release-field="next-step"
        className="mt-3 grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2"
      >
        <span className="text-muted text-xs">{t("colNextStep")}</span>
        <span className="text-text truncate text-sm">
          {release.next_step?.name ?? <span className="text-muted">{t("noNextStep")}</span>}
        </span>
      </div>
      <div
        data-release-field="progress"
        className="mt-2 grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2"
      >
        <span className="text-muted text-xs">{t("colProgress")}</span>
        <ReleaseProgress release={release} />
      </div>
      <div
        data-release-field="age"
        className="mt-2 grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2"
      >
        <span className="text-muted text-xs">{t("colAge")}</span>
        <time
          className="text-muted text-sm"
          dateTime={release.created_at}
          title={createdAt.toLocaleString(locale)}
        >
          {formatAge(release.created_at, locale)}
        </time>
      </div>
    </li>
  );
}
