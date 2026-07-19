"use client";

import { RefreshCw } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { Button } from "@/components/ui/Button";
import { Link } from "@/i18n/routing";
import type { AutomationRule, AutomationRun } from "@/lib/queries/automations";
import { teamPath } from "@/lib/team-routing";
import { cn } from "@/lib/utils";

const statusClasses: Record<string, string> = {
  succeeded: "text-st-res",
  completed: "text-st-res",
  processed: "text-st-res",
  running: "text-st-ack",
  failed: "text-sev-critical",
  ignored: "text-muted",
};

export function RunsView({
  isFetching,
  onRefresh,
  rules,
  runs,
  teamId,
}: {
  isFetching: boolean;
  onRefresh: () => void;
  rules: AutomationRule[];
  runs: AutomationRun[];
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const ruleNames = new Map(rules.map((rule) => [rule.id, rule.name]));

  if (runs.length === 0) {
    return (
      <section className="surface rounded-md px-6 py-14 text-center">
        <RefreshCw className="text-muted mx-auto h-8 w-8" aria-hidden="true" />
        <h3 className="text-text mt-4 font-semibold">{t("noRuns")}</h3>
        <p className="text-muted mx-auto mt-1 max-w-lg text-sm">{t("noRunsDescription")}</p>
        <Button className="mt-5" onClick={onRefresh} loading={isFetching}>
          <RefreshCw className="h-4 w-4" /> {t("refresh")}
        </Button>
      </section>
    );
  }

  return (
    <>
      <div className="mb-4 flex justify-end">
        <Button size="sm" onClick={onRefresh} loading={isFetching}>
          <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" /> {t("refresh")}
        </Button>
      </div>
      <div className="surface overflow-x-auto rounded-md">
        <table className="w-full min-w-[820px] text-left text-sm">
          <thead className="surface-subtle border-border border-b text-xs uppercase">
            <tr>
              {["colRun", "colRule", "colStatus", "colResult", "colStarted", "colDuration"].map(
                (column) => (
                  <th key={column} className="text-muted px-5 py-3.5 font-medium">
                    {t(column)}
                  </th>
                ),
              )}
            </tr>
          </thead>
          <tbody className="divide-border divide-y">
            {runs.map((run) => {
              const duration = run.finished_at
                ? Math.max(
                    0,
                    new Date(run.finished_at).getTime() - new Date(run.started_at).getTime(),
                  )
                : null;
              return (
                <tr key={run.id} className="hover:bg-white/[0.04]">
                  <td className="text-text px-5 py-4 font-mono text-xs" title={run.id}>
                    {run.id.slice(0, 8)}
                  </td>
                  <td className="text-text px-5 py-4">
                    {run.rule_id ? (ruleNames.get(run.rule_id) ?? t("deletedRule")) : t("noRule")}
                  </td>
                  <td className="px-5 py-4">
                    <span
                      className={cn(
                        "font-medium capitalize",
                        statusClasses[run.status] ?? "text-muted",
                      )}
                    >
                      {run.status}
                    </span>
                  </td>
                  <td className="px-5 py-4">
                    {run.incident_id ? (
                      <Link
                        href={teamPath(teamId, "incidents", run.incident_id)}
                        className="text-gold hover:text-gold-hover"
                      >
                        {t("openIncident")}
                      </Link>
                    ) : run.error_code ? (
                      <span className="text-sev-critical" title={run.error_code}>
                        {run.error_code}
                      </span>
                    ) : (
                      <span className="text-muted">—</span>
                    )}
                  </td>
                  <td className="text-muted px-5 py-4 whitespace-nowrap">
                    {new Intl.DateTimeFormat(locale, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    }).format(new Date(run.started_at))}
                  </td>
                  <td className="text-muted px-5 py-4 tabular-nums">
                    {duration === null ? t("inProgress") : t("durationMs", { duration })}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </>
  );
}
