import { ChevronRight } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { buttonClassNames } from "@/components/ui/Button";
import { Link } from "@/i18n/routing";
import type { ReleaseListItem } from "@/lib/queries/releases";
import { ReleaseStateChip } from "./ReleaseStateChip";

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
  return formatter.format(-Math.floor(days / 30), "month");
}

export function ReleaseTable({
  hrefFor,
  releases,
}: {
  hrefFor: (releaseId: string) => string;
  releases: ReleaseListItem[];
}) {
  const t = useTranslations("Releases");
  const locale = useLocale();

  return (
    <div className="surface overflow-x-auto rounded-md">
      <table className="w-full min-w-[980px] text-left text-sm">
        <thead className="surface-subtle border-border border-b text-xs uppercase">
          <tr>
            {["colRelease", "colState", "colProgress", "colNextStep", "colAge", "colBlockers"].map(
              (column) => (
                <th key={column} className="text-muted px-5 py-3.5 font-medium">
                  {t(column)}
                </th>
              ),
            )}
            <th className="text-muted px-5 py-3.5 text-right font-medium">
              <span className="sr-only">{t("colAction")}</span>
            </th>
          </tr>
        </thead>
        <tbody className="divide-border divide-y">
          {releases.map((release) => {
            const percentage =
              release.progress.total === 0
                ? 0
                : Math.round((release.progress.completed / release.progress.total) * 100);
            const createdAt = new Date(release.created_at);

            return (
              <tr
                key={release.release_id}
                className="group transition-colors hover:bg-white/[0.04]"
              >
                <td className="px-5 py-3.5 align-middle">
                  <Link
                    href={hrefFor(release.release_id)}
                    className="text-text hover:text-gold font-medium transition-colors"
                  >
                    {release.title}
                  </Link>
                </td>
                <td className="px-5 py-3.5 align-middle">
                  <ReleaseStateChip state={release.state} />
                </td>
                <td className="px-5 py-3.5 align-middle">
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
                </td>
                <td className="text-text max-w-56 px-5 py-3.5 align-middle">
                  {release.next_step?.name ?? <span className="text-muted">{t("noNextStep")}</span>}
                </td>
                <td
                  className="text-muted px-5 py-3.5 align-middle whitespace-nowrap"
                  title={createdAt.toLocaleString(locale)}
                >
                  {formatAge(release.created_at, locale)}
                </td>
                <td className="max-w-72 px-5 py-3.5 align-middle">
                  {release.blockers.length === 0 ? (
                    <span className="text-muted">—</span>
                  ) : (
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
                  )}
                </td>
                <td className="px-5 py-3.5 text-right align-middle">
                  <Link
                    href={hrefFor(release.release_id)}
                    className={buttonClassNames({ size: "sm" })}
                  >
                    {t("openRelease")} <ChevronRight className="h-3.5 w-3.5" />
                  </Link>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export function ReleaseTableSkeleton() {
  return (
    <div className="surface overflow-hidden rounded-md" aria-label="Loading releases">
      <div className="surface-subtle border-border h-11 border-b" />
      <div className="divide-border divide-y">
        {Array.from({ length: 5 }, (_, index) => (
          <div
            key={index}
            className="grid h-[73px] animate-pulse grid-cols-[2fr_8rem_10rem_1.5fr_7rem_2fr_5rem] items-center gap-5 px-5"
          >
            <span className="bg-panel-2 h-4 w-3/4 rounded" />
            <span className="bg-panel-2 h-5 w-20 rounded-full" />
            <span className="bg-panel-2 h-2 w-28 rounded-full" />
            <span className="bg-panel-2 h-4 w-4/5 rounded" />
            <span className="bg-panel-2 h-4 w-14 rounded" />
            <span className="bg-panel-2 h-4 w-4/5 rounded" />
            <span className="bg-panel-2 h-8 w-16 rounded-md" />
          </div>
        ))}
      </div>
    </div>
  );
}
