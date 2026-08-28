"use client";

import { CircleAlert, CircleCheck, CircleHelp, Loader, SkipForward } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import { TableFilterControl, TableSortControl } from "@/components/ui/CollectionControls";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "@/components/ui/OperationalTable";
import { Link } from "@/i18n/routing";
import type { AutomationRule, AutomationRun } from "@/lib/queries/automations";
import { teamPath } from "@/lib/team-routing";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useErrorText } from "@/lib/useErrorText";

const RUN_STATUS_KEYS = {
  running: "runStatusRunning",
  succeeded: "runStatusSucceeded",
  failed: "runStatusFailed",
  skipped: "runStatusSkipped",
} as const;

export function runStatusKey(status: string) {
  return RUN_STATUS_KEYS[status as keyof typeof RUN_STATUS_KEYS] ?? "runStatusUnknown";
}

export function RunStatus({ status }: { status: string }) {
  const t = useTranslations("Automations");

  switch (status) {
    case "running":
      return (
        <StatusBadge tone="info" icon={<Loader />}>
          {t("runStatusRunning")}
        </StatusBadge>
      );
    case "succeeded":
      return (
        <StatusBadge tone="success" icon={<CircleCheck />}>
          {t("runStatusSucceeded")}
        </StatusBadge>
      );
    case "failed":
      return (
        <StatusBadge tone="danger" icon={<CircleAlert />}>
          {t("runStatusFailed")}
        </StatusBadge>
      );
    case "skipped":
      return (
        <StatusBadge tone="neutral" icon={<SkipForward />}>
          {t("runStatusSkipped")}
        </StatusBadge>
      );
    default:
      return (
        <StatusBadge tone="neutral" icon={<CircleHelp />}>
          {t("runStatusUnknown")}
        </StatusBadge>
      );
  }
}

export function RunsView({
  rules,
  runs,
  teamId,
  ruleFilter = "all",
  sort = "started_desc",
  statusFilter = "all",
  showControls = false,
  onRuleFilterChange = () => undefined,
  onSortChange = () => undefined,
  onStatusFilterChange = () => undefined,
}: {
  rules: AutomationRule[];
  runs: AutomationRun[];
  teamId: string;
  ruleFilter?: string;
  sort?: "duration_asc" | "duration_desc" | "started_asc" | "started_desc";
  statusFilter?: string;
  showControls?: boolean;
  onRuleFilterChange?: (rule: string) => void;
  onSortChange?: (sort: "duration_asc" | "duration_desc" | "started_asc" | "started_desc") => void;
  onStatusFilterChange?: (status: string) => void;
}) {
  const t = useTranslations("Automations");
  const errorText = useErrorText();
  const locale = useLocale();
  const ruleNames = new Map(rules.map((rule) => [rule.id, rule.name]));
  const statuses = Array.from(new Set(runs.map((run) => run.status))).sort();
  const visibleRuns = runs
    .filter((run) => statusFilter === "all" || run.status === statusFilter)
    .filter((run) => ruleFilter === "all" || run.rule_id === ruleFilter)
    .toSorted((left, right) => {
      if (sort.startsWith("duration")) {
        const duration = (run: AutomationRun) =>
          run.finished_at
            ? new Date(run.finished_at).getTime() - new Date(run.started_at).getTime()
            : Infinity;
        return sort === "duration_asc"
          ? duration(left) - duration(right)
          : duration(right) - duration(left);
      }
      const delta = new Date(right.started_at).getTime() - new Date(left.started_at).getTime();
      return sort === "started_desc" ? delta : -delta;
    });

  if (runs.length === 0) {
    return (
      <section className="surface rounded-md p-12 text-center">
        <h3 className="text-muted text-sm font-medium">{t("noRuns")}</h3>
      </section>
    );
  }

  return (
    <OperationalTable
      label={t("runsList")}
      className="min-w-[820px]"
      containerClassName="overflow-x-auto"
    >
      <OperationalTableHead>
        <tr>
          <OperationalTableHeaderCell>{t("colRun")}</OperationalTableHeaderCell>
          <OperationalTableHeaderCell>
            {showControls ? (
              <TableFilterControl
                label={t("colStatus")}
                value={statusFilter === "all" ? "" : statusFilter}
                activeLabel={statusFilter === "all" ? undefined : t(runStatusKey(statusFilter))}
                onChange={(value) => onStatusFilterChange(value || "all")}
                options={[
                  { value: "", label: t("allStatuses") },
                  ...statuses.map((status) => ({
                    value: status,
                    label: t(runStatusKey(status)),
                  })),
                ]}
              />
            ) : (
              t("colStatus")
            )}
          </OperationalTableHeaderCell>
          <OperationalTableHeaderCell>
            {showControls ? (
              <TableFilterControl
                label={t("colRule")}
                value={ruleFilter === "all" ? "" : ruleFilter}
                activeLabel={ruleFilter === "all" ? undefined : ruleNames.get(ruleFilter)}
                onChange={(value) => onRuleFilterChange(value || "all")}
                options={[
                  { value: "", label: t("allRules") },
                  ...rules.map((rule) => ({ value: rule.id, label: rule.name })),
                ]}
              />
            ) : (
              t("colRule")
            )}
          </OperationalTableHeaderCell>
          <OperationalTableHeaderCell>{t("colResult")}</OperationalTableHeaderCell>
          <OperationalTableHeaderCell
            aria-sort={
              sort.startsWith("started")
                ? sort === "started_asc"
                  ? "ascending"
                  : "descending"
                : "none"
            }
          >
            {showControls ? (
              <TableSortControl
                label={t("colStarted")}
                direction={
                  sort.startsWith("started")
                    ? sort === "started_asc"
                      ? "ascending"
                      : "descending"
                    : undefined
                }
                onToggle={() =>
                  onSortChange(sort === "started_desc" ? "started_asc" : "started_desc")
                }
              />
            ) : (
              t("colStarted")
            )}
          </OperationalTableHeaderCell>
          <OperationalTableHeaderCell
            aria-sort={
              sort.startsWith("duration")
                ? sort === "duration_asc"
                  ? "ascending"
                  : "descending"
                : "none"
            }
          >
            {showControls ? (
              <TableSortControl
                label={t("colDuration")}
                direction={
                  sort.startsWith("duration")
                    ? sort === "duration_asc"
                      ? "ascending"
                      : "descending"
                    : undefined
                }
                onToggle={() =>
                  onSortChange(sort === "duration_asc" ? "duration_desc" : "duration_asc")
                }
              />
            ) : (
              t("colDuration")
            )}
          </OperationalTableHeaderCell>
        </tr>
      </OperationalTableHead>
      <OperationalTableBody>
        {visibleRuns.map((run) => {
          const duration = run.finished_at
            ? Math.max(0, new Date(run.finished_at).getTime() - new Date(run.started_at).getTime())
            : null;
          return (
            <OperationalTableRow key={run.id}>
              <OperationalTableRowHeader className="text-text font-mono text-xs" title={run.id}>
                {run.id.slice(0, 8)}
              </OperationalTableRowHeader>
              <OperationalTableCell>
                <RunStatus status={run.status} />
              </OperationalTableCell>
              <OperationalTableCell className="text-text">
                {run.rule_id ? (ruleNames.get(run.rule_id) ?? t("deletedRule")) : t("noRule")}
              </OperationalTableCell>
              <OperationalTableCell>
                {run.incident_id ? (
                  <Link
                    href={teamPath(teamId, "incidents", run.incident_id)}
                    className="text-gold hover:text-gold-hover"
                  >
                    {t("openIncident")}
                  </Link>
                ) : run.error_code ? (
                  <span className="text-sev-critical" title={errorText(run.error_code)}>
                    {errorText(run.error_code)}
                  </span>
                ) : (
                  <span className="text-muted">—</span>
                )}
              </OperationalTableCell>
              <OperationalTableCell className="text-muted whitespace-nowrap">
                {new Intl.DateTimeFormat(locale, {
                  dateStyle: "medium",
                  timeStyle: "short",
                }).format(new Date(run.started_at))}
              </OperationalTableCell>
              <OperationalTableCell className="text-muted tabular-nums">
                {duration === null ? t("inProgress") : t("durationMs", { duration })}
              </OperationalTableCell>
            </OperationalTableRow>
          );
        })}
      </OperationalTableBody>
    </OperationalTable>
  );
}
