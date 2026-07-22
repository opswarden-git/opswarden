"use client";

import { Pencil, Plus, Power, PowerOff, Trash2 } from "lucide-react";
import { useLocale, useTranslations } from "next-intl";
import React, { useMemo, useRef, useState } from "react";
import { Alert } from "@/components/ui/Alert";
import { ActionMenu } from "@/components/ui/ActionMenu";
import { Button } from "@/components/ui/Button";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import {
  type AutomationRule,
  type AutomationRuleDefinition,
  type AutomationService,
  type CatalogCapability,
  type TeamConnection,
  useCreateAutomationRule,
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
} from "@/components/ui/OperationalTable";
import { AutomationDialog } from "./AutomationDialog";

const FILTER_FIELDS = ["repository", "workflow", "branch", "conclusion"] as const;
const SEVERITIES = ["low", "medium", "high", "critical"] as const;

type CapabilityWithService = CatalogCapability & { service: string };

function capabilities(catalog: AutomationService[], type: "actions" | "reactions") {
  return catalog.flatMap((service) =>
    service[type].map((capability) => ({ ...capability, service: service.name })),
  );
}

function capabilityLabel(options: CapabilityWithService[], name: string, fallback: string) {
  return options.find((option) => option.name === name)?.label ?? fallback;
}

function RuleForm({
  actions,
  connections,
  onClose,
  reactions,
  rule,
  teamId,
}: {
  actions: CapabilityWithService[];
  connections: TeamConnection[];
  onClose: () => void;
  reactions: CapabilityWithService[];
  rule?: AutomationRule;
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const nameRef = useRef<HTMLInputElement>(null);
  const [name, setName] = useState(rule?.name ?? "");
  const [actionName, setActionName] = useState(rule?.trigger_kind ?? actions[0]?.name ?? "");
  const [triggerConnectionId, setTriggerConnectionId] = useState(rule?.trigger_connection_id ?? "");
  const [reactionName, setReactionName] = useState(rule?.reaction_kind ?? reactions[0]?.name ?? "");
  const [reactionConnectionId, setReactionConnectionId] = useState(
    rule?.reaction_connection_id ?? "",
  );
  const [filters, setFilters] = useState<Record<string, string>>(
    Object.fromEntries(
      FILTER_FIELDS.map((field) => [field, String(rule?.trigger_config[field] ?? "")]),
    ),
  );
  const [severity, setSeverity] = useState(String(rule?.reaction_config.severity ?? "high"));
  const [incidentTitle, setIncidentTitle] = useState(String(rule?.reaction_config.title ?? ""));
  const createRule = useCreateAutomationRule(teamId);
  const updateRule = useUpdateAutomationRule(teamId);
  const mutation = rule ? updateRule : createRule;

  const selectedAction = actions.find((action) => action.name === actionName);
  const selectedReaction = reactions.find((reaction) => reaction.name === reactionName);
  const triggerConnections = connections.filter(
    (connection) => connection.service === selectedAction?.connection_service,
  );
  const reactionConnections = selectedReaction?.connection_service
    ? connections.filter((connection) => connection.service === selectedReaction.connection_service)
    : [];
  const needsReactionConnection = !!selectedReaction?.connection_service;
  const valid =
    !!name.trim() &&
    !!selectedAction &&
    !!triggerConnectionId &&
    !!selectedReaction &&
    (!needsReactionConnection || !!reactionConnectionId);

  const selectAction = (nextName: string) => {
    setActionName(nextName);
    const next = actions.find((action) => action.name === nextName);
    if (
      connections.find((item) => item.id === triggerConnectionId)?.service !==
      next?.connection_service
    ) {
      setTriggerConnectionId("");
    }
  };

  const selectReaction = (nextName: string) => {
    setReactionName(nextName);
    const next = reactions.find((reaction) => reaction.name === nextName);
    if (!next?.connection_service) setReactionConnectionId("");
    else if (
      connections.find((item) => item.id === reactionConnectionId)?.service !==
      next.connection_service
    ) {
      setReactionConnectionId("");
    }
  };

  const definition = (): AutomationRuleDefinition => ({
    name: name.trim(),
    trigger_connection_id: triggerConnectionId,
    trigger_kind: actionName,
    trigger_config: Object.fromEntries(
      Object.entries(filters).filter(([, value]) => value.trim().length > 0),
    ),
    reaction_kind: reactionName,
    reaction_connection_id: needsReactionConnection ? reactionConnectionId : null,
    reaction_config:
      reactionName === "vigil_create_incident"
        ? {
            severity,
            ...(incidentTitle.trim() ? { title: incidentTitle.trim() } : {}),
          }
        : {},
  });

  return (
    <AutomationDialog
      open
      onClose={onClose}
      initialFocus={nameRef}
      title={rule ? t("editRule") : t("newRule")}
      description={t("ruleFormDescription")}
      footer={
        <>
          <Button size="lg" onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button
            type="submit"
            form="rule-form"
            size="lg"
            variant="primary"
            disabled={!valid}
            loading={mutation.isPending}
          >
            {rule ? t("saveChanges") : t("createRule")}
          </Button>
        </>
      }
    >
      <form
        id="rule-form"
        className="space-y-6 p-6"
        onSubmit={(event) => {
          event.preventDefault();
          if (!valid) return;
          if (rule) {
            updateRule.mutate({ ruleId: rule.id, ...definition() }, { onSuccess: onClose });
          } else {
            createRule.mutate(definition(), { onSuccess: onClose });
          }
        }}
      >
        <label className="text-text block text-sm font-medium">
          <span>{t("ruleName")}</span>
          <input
            ref={nameRef}
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            placeholder={t("ruleNamePlaceholder")}
            maxLength={200}
            required
          />
        </label>

        <fieldset className="surface-subtle border-border space-y-4 rounded-md border p-4">
          <legend className="text-text px-1 text-sm font-semibold">{t("action")}</legend>
          <label className="text-text block text-sm font-medium">
            <span>{t("event")}</span>
            <select
              value={actionName}
              onChange={(event) => selectAction(event.target.value)}
              className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            >
              {actions.map((action) => (
                <option key={action.name} value={action.name}>
                  {action.label}
                </option>
              ))}
            </select>
          </label>
          <label className="text-text block text-sm font-medium">
            <span>{t("sourceConnection")}</span>
            <select
              value={triggerConnectionId}
              onChange={(event) => setTriggerConnectionId(event.target.value)}
              className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
              required
            >
              <option value="">{t("selectConnection")}</option>
              {triggerConnections.map((connection) => (
                <option key={connection.id} value={connection.id}>
                  {selectedAction?.service} · {connection.id.slice(0, 8)}
                </option>
              ))}
            </select>
          </label>
          {triggerConnections.length === 0 ? (
            <Alert tone="warning">{t("missingSourceConnection")}</Alert>
          ) : null}
          <div>
            <div className="text-text text-sm font-medium">{t("optionalFilters")}</div>
            <p className="text-muted mt-1 text-xs">{t("filtersHint")}</p>
            <div className="mt-3 grid gap-3 sm:grid-cols-2">
              {FILTER_FIELDS.map((field) => (
                <label key={field} className="text-muted block text-xs font-medium capitalize">
                  <span>{field}</span>
                  <input
                    value={filters[field]}
                    onChange={(event) =>
                      setFilters((current) => ({ ...current, [field]: event.target.value }))
                    }
                    className="ow-input mt-1.5 h-9 w-full rounded-md px-3 text-sm normal-case"
                  />
                </label>
              ))}
            </div>
          </div>
        </fieldset>

        <fieldset className="surface-subtle border-border space-y-4 rounded-md border p-4">
          <legend className="text-text px-1 text-sm font-semibold">{t("reaction")}</legend>
          <label className="text-text block text-sm font-medium">
            <span>{t("outcome")}</span>
            <select
              value={reactionName}
              onChange={(event) => selectReaction(event.target.value)}
              className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            >
              {reactions.map((reaction) => (
                <option key={reaction.name} value={reaction.name}>
                  {reaction.label}
                </option>
              ))}
            </select>
          </label>
          {needsReactionConnection ? (
            <>
              <label className="text-text block text-sm font-medium">
                <span>{t("destinationConnection")}</span>
                <select
                  value={reactionConnectionId}
                  onChange={(event) => setReactionConnectionId(event.target.value)}
                  className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
                  required
                >
                  <option value="">{t("selectConnection")}</option>
                  {reactionConnections.map((connection) => (
                    <option key={connection.id} value={connection.id}>
                      {selectedReaction?.service} · {connection.id.slice(0, 8)}
                    </option>
                  ))}
                </select>
              </label>
              {reactionConnections.length === 0 ? (
                <Alert tone="warning">{t("missingDestinationConnection")}</Alert>
              ) : null}
            </>
          ) : null}
          {reactionName === "vigil_create_incident" ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="text-text block text-sm font-medium">
                <span>{t("incidentSeverity")}</span>
                <select
                  value={severity}
                  onChange={(event) => setSeverity(event.target.value)}
                  className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
                >
                  {SEVERITIES.map((value) => (
                    <option key={value} value={value}>
                      {t(`severity.${value}`)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="text-text block text-sm font-medium">
                <span>{t("incidentTitleOptional")}</span>
                <input
                  value={incidentTitle}
                  onChange={(event) => setIncidentTitle(event.target.value)}
                  className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
                  maxLength={200}
                />
              </label>
            </div>
          ) : null}
        </fieldset>

        <Alert tone="info">{t("savedDisabledHint")}</Alert>
        {mutation.error ? (
          <Alert tone="danger">{t("requestFailed", { code: mutation.error.message })}</Alert>
        ) : null}
      </form>
    </AutomationDialog>
  );
}

export function RulesView({
  catalog,
  connections,
  rules,
  teamId,
  isCreatingRule,
  setIsCreatingRule,
}: {
  catalog: AutomationService[];
  connections: TeamConnection[];
  rules: AutomationRule[];
  teamId: string;
  isCreatingRule: boolean;
  setIsCreatingRule: (creating: boolean) => void;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const [editing, setEditing] = useState<AutomationRule | null>(null);
  const [deleting, setDeleting] = useState<AutomationRule | null>(null);
  const updateRule = useUpdateAutomationRule(teamId);
  const deleteRule = useDeleteAutomationRule(teamId);
  const actions = useMemo(() => capabilities(catalog, "actions"), [catalog]);
  const reactions = useMemo(() => capabilities(catalog, "reactions"), [catalog]);

  if (rules.length === 0) {
    return (
      <>
        <section className="surface rounded-md px-6 py-14 text-center">
          <Power className="text-muted mx-auto h-8 w-8" aria-hidden="true" />
          <h3 className="text-text mt-4 font-semibold">{t("noRules")}</h3>
          <p className="text-muted mx-auto mt-1 max-w-lg text-sm">{t("noRulesDescription")}</p>
        </section>
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
          {t("requestFailed", { code: updateRule.error.message })}
        </Alert>
      ) : null}

      {/* Desktop view */}
      <div className="hidden pt-6 lg:block">
        <OperationalTable label={t("rulesList")}>
          <OperationalTableHead>
            <tr>
              {["colRule", "colAction", "colReaction", "colStatus", "colUpdated"].map((column) => (
                <OperationalTableHeaderCell key={column}>{t(column)}</OperationalTableHeaderCell>
              ))}
              <th className="px-5 py-3.5">
                <span className="sr-only">{t("actionsMenu")}</span>
              </th>
            </tr>
          </OperationalTableHead>
          <OperationalTableBody>
            {rules.map((rule) => (
              <OperationalTableRow key={rule.id}>
                <OperationalTableCell className="text-text font-medium">
                  {rule.name}
                </OperationalTableCell>
                <OperationalTableCell className="text-muted">
                  {capabilityLabel(actions, rule.trigger_kind, rule.trigger_kind)}
                </OperationalTableCell>
                <OperationalTableCell className="text-muted">
                  {capabilityLabel(reactions, rule.reaction_kind, rule.reaction_kind)}
                </OperationalTableCell>
                <OperationalTableCell>
                  <span className={rule.enabled ? "text-st-res" : "text-muted"}>
                    {rule.enabled ? t("enabled") : t("disabled")}
                  </span>
                </OperationalTableCell>
                <OperationalTableCell className="text-muted whitespace-nowrap">
                  {new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                    new Date(rule.updated_at),
                  )}
                </OperationalTableCell>
                <OperationalTableCell className="text-right">
                  <ActionMenu
                    label={t("actionsMenu")}
                    disabled={updateRule.isPending}
                    items={[
                      {
                        id: "toggle",
                        label: rule.enabled ? t("disable") : t("enable"),
                        icon: rule.enabled ? PowerOff : Power,
                        onSelect: () =>
                          updateRule.mutate({ ruleId: rule.id, enabled: !rule.enabled }),
                      },
                      {
                        id: "edit",
                        label: t("edit"),
                        icon: Pencil,
                        onSelect: () => setEditing(rule),
                      },
                      { id: "separator", separator: true },
                      {
                        id: "delete",
                        label: t("delete"),
                        icon: Trash2,
                        tone: "danger",
                        onSelect: () => setDeleting(rule),
                      },
                    ]}
                  />
                </OperationalTableCell>
              </OperationalTableRow>
            ))}
          </OperationalTableBody>
        </OperationalTable>
      </div>

      {/* Mobile view */}
      <div className="surface overflow-hidden rounded-md pt-6 lg:hidden">
        <ul aria-label={t("rulesList")} className="divide-border divide-y">
          {rules.map((rule) => (
            <li key={rule.id} className="flex flex-col gap-3 p-4">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <h3 className="text-text font-medium">{rule.name}</h3>
                  <div className="mt-1 flex flex-wrap gap-2 text-sm">
                    <span className={rule.enabled ? "text-st-res" : "text-muted"}>
                      {rule.enabled ? t("enabled") : t("disabled")}
                    </span>
                    <span className="text-muted">•</span>
                    <span className="text-muted">
                      {new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                        new Date(rule.updated_at),
                      )}
                    </span>
                  </div>
                </div>
                <div className="shrink-0">
                  <ActionMenu
                    label={t("actionsMenu")}
                    disabled={updateRule.isPending}
                    items={[
                      {
                        id: "toggle",
                        label: rule.enabled ? t("disable") : t("enable"),
                        icon: rule.enabled ? PowerOff : Power,
                        onSelect: () =>
                          updateRule.mutate({ ruleId: rule.id, enabled: !rule.enabled }),
                      },
                      {
                        id: "edit",
                        label: t("edit"),
                        icon: Pencil,
                        onSelect: () => setEditing(rule),
                      },
                      { id: "separator", separator: true },
                      {
                        id: "delete",
                        label: t("delete"),
                        icon: Trash2,
                        tone: "danger",
                        onSelect: () => setDeleting(rule),
                      },
                    ]}
                  />
                </div>
              </div>
              <div className="surface-subtle border-border rounded border px-3 py-2 text-sm">
                <div className="flex flex-col gap-1">
                  <div className="flex justify-between gap-4">
                    <span className="text-muted shrink-0 text-xs uppercase">{t("colAction")}</span>
                    <span className="text-text truncate text-right">
                      {capabilityLabel(actions, rule.trigger_kind, rule.trigger_kind)}
                    </span>
                  </div>
                  <div className="flex justify-between gap-4">
                    <span className="text-muted shrink-0 text-xs uppercase">
                      {t("colReaction")}
                    </span>
                    <span className="text-text truncate text-right">
                      {capabilityLabel(reactions, rule.reaction_kind, rule.reaction_kind)}
                    </span>
                  </div>
                </div>
              </div>
            </li>
          ))}
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
        danger
        pending={deleteRule.isPending}
        error={deleteRule.error ? t("requestFailed", { code: deleteRule.error.message }) : null}
        onClose={() => setDeleting(null)}
        onConfirm={() =>
          deleting && deleteRule.mutate(deleting.id, { onSuccess: () => setDeleting(null) })
        }
      />
    </>
  );
}
