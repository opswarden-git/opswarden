"use client";

import { Pencil, Plus, Power, PowerOff, Trash2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import React, { useMemo, useState } from "react";
import { Alert } from "@/components/ui/Alert";
import { ActionMenu } from "@/components/ui/ActionMenu";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { TableFilterControl, TableSortControl } from "@/components/ui/CollectionControls";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { StatusBadge } from "@/components/ui/StatusBadge";
import {
  type AutomationRule,
  type AutomationService,
  type TeamConnection,
  useDeleteAutomationRule,
  useUpdateAutomationRule,
} from "@/lib/queries/automations";
import {
  OperationalTable,
  OperationalTableBody,
  OperationalTableCell,
  OperationalTableHead,
  OperationalTableHeaderCell,
  OperationalTableRow,
  OperationalTableRowHeader,
} from "@/components/ui/OperationalTable";
import type { CapabilityWithService } from "@/lib/automation-catalog";
import { RuleForm } from "./RuleForm";
import { useErrorText } from "@/lib/useErrorText";

function capabilities(catalog: AutomationService[], type: "actions" | "reactions") {
  return catalog.flatMap((service) =>
    service[type].map((capability) => ({
      ...capability,
      service: service.name,
      builtIn: service.connection === null,
    })),
  );
}

function capabilityLabel(options: CapabilityWithService[], name: string, fallback: string) {
  return options.find((option) => option.name === name)?.label ?? fallback;
}

function nextRunLabel(rule: AutomationRule, locale: string, disabledLabel: string) {
  if (!rule.enabled) return disabledLabel;
  if (!rule.next_run_at) return "—";
  const timezone =
    typeof rule.trigger_config.timezone === "string" ? rule.trigger_config.timezone : undefined;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: timezone,
  }).format(new Date(rule.next_run_at));
}

export interface RuleProjection {
  rule: AutomationRule;
  triggerLabel: string;
  reactionLabel: string;
  nextRunLabel: string;
  updatedAtLabel: string;
}

export function projectRule(
  rule: AutomationRule,
  actions: CapabilityWithService[],
  reactions: CapabilityWithService[],
  locale: string,
  disabledLabel: string,
): RuleProjection {
  return {
    rule,
    triggerLabel: capabilityLabel(actions, rule.trigger_kind, rule.trigger_kind),
    reactionLabel: capabilityLabel(reactions, rule.reaction_kind, rule.reaction_kind),
    nextRunLabel: nextRunLabel(rule, locale, disabledLabel),
    updatedAtLabel: new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
      new Date(rule.updated_at),
    ),
  };
}

function RuleStatus({ enabled }: { enabled: boolean }) {
  const t = useTranslations("Automations");
  return enabled ? (
    <StatusBadge tone="success" icon={<Power />}>
      {t("enabled")}
    </StatusBadge>
  ) : (
    <StatusBadge tone="neutral" icon={<PowerOff />}>
      {t("disabled")}
    </StatusBadge>
  );
}

function RuleActionMenu({
  rule,
  disabled,
  label,
  enableLabel,
  disableLabel,
  editLabel,
  deleteLabel,
  onToggle,
  onEdit,
  onDelete,
}: {
  rule: AutomationRule;
  disabled: boolean;
  label: string;
  enableLabel: string;
  disableLabel: string;
  editLabel: string;
  deleteLabel: string;
  onToggle: (rule: AutomationRule) => void;
  onEdit: (rule: AutomationRule) => void;
  onDelete: (rule: AutomationRule) => void;
}) {
  return (
    <ActionMenu
      label={label}
      disabled={disabled}
      items={[
        {
          id: "toggle",
          label: rule.enabled ? disableLabel : enableLabel,
          icon: rule.enabled ? PowerOff : Power,
          onSelect: () => onToggle(rule),
        },
        {
          id: "edit",
          label: editLabel,
          icon: Pencil,
          onSelect: () => onEdit(rule),
        },
        { id: "separator", separator: true },
        {
          id: "delete",
          label: deleteLabel,
          icon: Trash2,
          tone: "danger",
          onSelect: () => onDelete(rule),
        },
      ]}
    />
  );
}

export function RulesView({
  catalog,
  connections,
  rules,
  teamId,
  isCreatingRule,
  setIsCreatingRule,
  sort = "updated_desc",
  statusFilter = "all",
  onSortChange = () => undefined,
  onStatusFilterChange = () => undefined,
}: {
  catalog: AutomationService[];
  connections: TeamConnection[];
  rules: AutomationRule[];
  teamId: string;
  isCreatingRule: boolean;
  setIsCreatingRule: (creating: boolean) => void;
  sort?: "next_asc" | "next_desc" | "updated_asc" | "updated_desc";
  statusFilter?: "all" | "enabled" | "disabled";
  onSortChange?: (sort: "next_asc" | "next_desc" | "updated_asc" | "updated_desc") => void;
  onStatusFilterChange?: (status: "all" | "enabled" | "disabled") => void;
}) {
  const t = useTranslations("Automations");
  const errorText = useErrorText();
  const locale = useLocale();
  const [editing, setEditing] = useState<AutomationRule | null>(null);
  const [deleting, setDeleting] = useState<AutomationRule | null>(null);
  const updateRule = useUpdateAutomationRule(teamId);
  const deleteRule = useDeleteAutomationRule(teamId);
  const actions = useMemo(() => capabilities(catalog, "actions"), [catalog]);
  const reactions = useMemo(() => capabilities(catalog, "reactions"), [catalog]);
  const visibleRules = useMemo(() => {
    const filtered = rules.filter(
      (rule) =>
        statusFilter === "all" || (statusFilter === "enabled" ? rule.enabled : !rule.enabled),
    );
    return filtered.toSorted((left, right) => {
      if (sort.startsWith("next")) {
        const leftTime = left.next_run_at ? new Date(left.next_run_at).getTime() : Infinity;
        const rightTime = right.next_run_at ? new Date(right.next_run_at).getTime() : Infinity;
        return sort === "next_asc" ? leftTime - rightTime : rightTime - leftTime;
      }
      const delta = new Date(right.updated_at).getTime() - new Date(left.updated_at).getTime();
      return sort === "updated_desc" ? delta : -delta;
    });
  }, [rules, sort, statusFilter]);

  const projectedRules = useMemo(() => {
    const disabledText = t("disabled");
    return visibleRules.map((rule) => projectRule(rule, actions, reactions, locale, disabledText));
  }, [visibleRules, actions, reactions, locale, t]);

  if (rules.length === 0) {
    return (
      <>
        <EmptyState
          icon={<Power className="h-5 w-5" />}
          title={t("noRules")}
          description={t("noRulesDescription")}
        />
        {isCreatingRule ? (
          <RuleForm
            teamId={teamId}
            actions={actions}
            reactions={reactions}
            connections={connections}
            onClose={() => setIsCreatingRule(false)}
          />
        ) : null}
      </>
    );
  }

  return (
    <>
      {updateRule.error ? (
        <Alert tone="danger" className="mb-4">
          {t("requestFailed", { code: errorText(updateRule.error.message) })}
        </Alert>
      ) : null}

      {/* Desktop view */}
      <div className="hidden lg:block">
        <OperationalTable label={t("rulesList")} containerClassName="overflow-x-auto">
          <OperationalTableHead>
            <tr>
              <OperationalTableHeaderCell>{t("colRule")}</OperationalTableHeaderCell>
              <OperationalTableHeaderCell>
                <TableFilterControl
                  label={t("colStatus")}
                  value={statusFilter}
                  activeLabel={statusFilter === "all" ? undefined : t(statusFilter)}
                  onChange={(value) =>
                    onStatusFilterChange(value as "all" | "enabled" | "disabled")
                  }
                  options={[
                    { value: "all", label: t("allStatuses") },
                    { value: "enabled", label: t("enabled") },
                    { value: "disabled", label: t("disabled") },
                  ]}
                />
              </OperationalTableHeaderCell>
              <OperationalTableHeaderCell>{t("colTrigger")}</OperationalTableHeaderCell>
              <OperationalTableHeaderCell>{t("colResponse")}</OperationalTableHeaderCell>
              <OperationalTableHeaderCell
                className="whitespace-nowrap"
                aria-sort={
                  sort.startsWith("next")
                    ? sort === "next_asc"
                      ? "ascending"
                      : "descending"
                    : "none"
                }
              >
                <TableSortControl
                  label={t("colNextRun")}
                  direction={
                    sort.startsWith("next")
                      ? sort === "next_asc"
                        ? "ascending"
                        : "descending"
                      : undefined
                  }
                  onToggle={() => onSortChange(sort === "next_asc" ? "next_desc" : "next_asc")}
                />
              </OperationalTableHeaderCell>
              <OperationalTableHeaderCell
                aria-sort={
                  sort.startsWith("updated")
                    ? sort === "updated_asc"
                      ? "ascending"
                      : "descending"
                    : "none"
                }
              >
                <TableSortControl
                  label={t("colUpdated")}
                  direction={
                    sort.startsWith("updated")
                      ? sort === "updated_asc"
                        ? "ascending"
                        : "descending"
                      : undefined
                  }
                  onToggle={() =>
                    onSortChange(sort === "updated_desc" ? "updated_asc" : "updated_desc")
                  }
                />
              </OperationalTableHeaderCell>
              <th className="px-4 py-3">
                <span className="sr-only">{t("actionsMenu")}</span>
              </th>
            </tr>
          </OperationalTableHead>
          <OperationalTableBody>
            {projectedRules.map(
              ({ rule, triggerLabel, reactionLabel, nextRunLabel, updatedAtLabel }) => (
                <OperationalTableRow key={rule.id}>
                  <OperationalTableRowHeader className="text-text font-medium">
                    {rule.name}
                  </OperationalTableRowHeader>
                  <OperationalTableCell>
                    <span data-rule-state={rule.enabled ? "enabled" : "disabled"}>
                      <RuleStatus enabled={rule.enabled} />
                    </span>
                  </OperationalTableCell>
                  <OperationalTableCell className="text-muted">{triggerLabel}</OperationalTableCell>
                  <OperationalTableCell className="text-muted">
                    {reactionLabel}
                  </OperationalTableCell>
                  <OperationalTableCell className="text-muted whitespace-nowrap">
                    {nextRunLabel}
                  </OperationalTableCell>
                  <OperationalTableCell className="text-muted whitespace-nowrap">
                    {updatedAtLabel}
                  </OperationalTableCell>
                  <OperationalTableCell className="text-right">
                    <RuleActionMenu
                      rule={rule}
                      disabled={updateRule.isPending}
                      label={t("actionsMenu")}
                      enableLabel={t("enable")}
                      disableLabel={t("disable")}
                      editLabel={t("edit")}
                      deleteLabel={t("delete")}
                      onToggle={(r) => updateRule.mutate({ ruleId: r.id, enabled: !r.enabled })}
                      onEdit={setEditing}
                      onDelete={setDeleting}
                    />
                  </OperationalTableCell>
                </OperationalTableRow>
              ),
            )}
          </OperationalTableBody>
        </OperationalTable>
      </div>

      {/* Mobile view */}
      <div className="surface overflow-hidden rounded-md lg:hidden">
        <ul aria-label={t("rulesList")} className="divide-border-muted divide-y">
          {projectedRules.map(
            ({ rule, triggerLabel, reactionLabel, nextRunLabel, updatedAtLabel }) => (
              <li key={rule.id} className="flex flex-col gap-3 p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <h3 className="text-text font-medium">{rule.name}</h3>
                    <div className="mt-1 flex flex-wrap text-sm">
                      <span data-rule-state={rule.enabled ? "enabled" : "disabled"}>
                        <RuleStatus enabled={rule.enabled} />
                      </span>
                    </div>
                  </div>
                  <div className="shrink-0">
                    <RuleActionMenu
                      rule={rule}
                      disabled={updateRule.isPending}
                      label={t("actionsMenu")}
                      enableLabel={t("enable")}
                      disableLabel={t("disable")}
                      editLabel={t("edit")}
                      deleteLabel={t("delete")}
                      onToggle={(r) => updateRule.mutate({ ruleId: r.id, enabled: !r.enabled })}
                      onEdit={setEditing}
                      onDelete={setDeleting}
                    />
                  </div>
                </div>
                <div className="surface-subtle border-border rounded border px-3 py-2 text-sm">
                  <div className="flex flex-col gap-1">
                    <div className="flex justify-between gap-4">
                      <span className="text-muted shrink-0 text-xs uppercase">
                        {t("colTrigger")}
                      </span>
                      <span className="text-text truncate text-right">{triggerLabel}</span>
                    </div>
                    <div className="flex justify-between gap-4">
                      <span className="text-muted shrink-0 text-xs uppercase">
                        {t("colResponse")}
                      </span>
                      <span className="text-text truncate text-right">{reactionLabel}</span>
                    </div>
                    <div className="flex justify-between gap-4">
                      <span className="text-muted shrink-0 text-xs uppercase">
                        {t("colNextRun")}
                      </span>
                      <span className="text-text text-right">{nextRunLabel}</span>
                    </div>
                    <div className="flex justify-between gap-4">
                      <span className="text-muted shrink-0 text-xs uppercase">
                        {t("colUpdated")}
                      </span>
                      <span className="text-text text-right">{updatedAtLabel}</span>
                    </div>
                  </div>
                </div>
              </li>
            ),
          )}
        </ul>
      </div>

      {isCreatingRule || editing ? (
        <RuleForm
          teamId={teamId}
          actions={actions}
          reactions={reactions}
          connections={connections}
          rule={editing ?? undefined}
          onClose={() => {
            setEditing(null);
            setIsCreatingRule(false);
          }}
        />
      ) : null}
      <ConfirmDialog
        open={!!deleting}
        title={t("deleteRuleTitle", { name: deleting?.name ?? "" })}
        description={t("deleteRuleDescription")}
        confirmLabel={t("delete")}
        cancelLabel={t("cancel")}
        intent="destructive"
        pending={deleteRule.isPending}
        error={
          deleteRule.error
            ? t("requestFailed", { code: errorText(deleteRule.error.message) })
            : null
        }
        onClose={() => setDeleting(null)}
        onConfirm={() =>
          deleting && deleteRule.mutate(deleting.id, { onSuccess: () => setDeleting(null) })
        }
      />
    </>
  );
}
