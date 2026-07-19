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
    >
      <form
        className="min-h-0 space-y-6 overflow-y-auto p-6"
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
        <div className="border-border flex justify-end gap-2 border-t pt-5">
          <Button size="lg" onClick={onClose}>
            {t("cancel")}
          </Button>
          <Button
            type="submit"
            size="lg"
            variant="primary"
            disabled={!valid}
            loading={mutation.isPending}
          >
            {rule ? t("saveChanges") : t("createRule")}
          </Button>
        </div>
      </form>
    </AutomationDialog>
  );
}

export function RulesView({
  catalog,
  connections,
  rules,
  teamId,
}: {
  catalog: AutomationService[];
  connections: TeamConnection[];
  rules: AutomationRule[];
  teamId: string;
}) {
  const t = useTranslations("Automations");
  const locale = useLocale();
  const [editing, setEditing] = useState<AutomationRule | "new" | null>(null);
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
          <Button className="mt-5" variant="primary" onClick={() => setEditing("new")}>
            <Plus className="h-4 w-4" /> {t("newRule")}
          </Button>
        </section>
        {editing === "new" ? (
          <RuleForm
            teamId={teamId}
            actions={actions}
            reactions={reactions}
            connections={connections}
            onClose={() => setEditing(null)}
          />
        ) : null}
      </>
    );
  }

  return (
    <>
      <div className="mb-4 flex justify-end">
        <Button variant="primary" onClick={() => setEditing("new")}>
          <Plus className="h-4 w-4" aria-hidden="true" /> {t("newRule")}
        </Button>
      </div>
      {updateRule.error ? (
        <Alert tone="danger" className="mb-4">
          {t("requestFailed", { code: updateRule.error.message })}
        </Alert>
      ) : null}
      <div className="surface overflow-x-auto rounded-md">
        <table className="w-full min-w-[820px] text-left text-sm">
          <thead className="surface-subtle border-border border-b text-xs uppercase">
            <tr>
              {["colRule", "colAction", "colReaction", "colStatus", "colUpdated"].map((column) => (
                <th key={column} className="text-muted px-5 py-3.5 font-medium">
                  {t(column)}
                </th>
              ))}
              <th className="px-5 py-3.5">
                <span className="sr-only">{t("actionsMenu")}</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-border divide-y">
            {rules.map((rule) => (
              <tr key={rule.id} className="hover:bg-white/[0.04]">
                <td className="text-text px-5 py-4 font-medium">{rule.name}</td>
                <td className="text-muted px-5 py-4">
                  {capabilityLabel(actions, rule.trigger_kind, rule.trigger_kind)}
                </td>
                <td className="text-muted px-5 py-4">
                  {capabilityLabel(reactions, rule.reaction_kind, rule.reaction_kind)}
                </td>
                <td className="px-5 py-4">
                  <span className={rule.enabled ? "text-st-res" : "text-muted"}>
                    {rule.enabled ? t("enabled") : t("disabled")}
                  </span>
                </td>
                <td className="text-muted px-5 py-4 whitespace-nowrap">
                  {new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
                    new Date(rule.updated_at),
                  )}
                </td>
                <td className="px-5 py-4 text-right">
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
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {editing ? (
        <RuleForm
          teamId={teamId}
          actions={actions}
          reactions={reactions}
          connections={connections}
          rule={editing === "new" ? undefined : editing}
          onClose={() => setEditing(null)}
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
