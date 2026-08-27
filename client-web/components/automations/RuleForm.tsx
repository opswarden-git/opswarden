"use client";

import { useTranslations } from "next-intl";
import React, { useRef, useState } from "react";
import { catalogFieldsAreValid, catalogPayload, catalogValues } from "@/lib/automation-catalog";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import {
  type AutomationRule,
  type AutomationRuleDefinition,
  type CatalogCapability,
  type TeamConnection,
  useCreateAutomationRule,
  useUpdateAutomationRule,
} from "@/lib/queries/automations";
import { AutomationDialog } from "./AutomationDialog";

export type CapabilityWithService = CatalogCapability & { service: string; builtIn: boolean };

function capabilityOptionValue(
  capabilities: CapabilityWithService[],
  capability: CapabilityWithService,
) {
  const duplicated = capabilities.some(
    (candidate) => candidate !== capability && candidate.name === capability.name,
  );
  return duplicated ? `${capability.service}:${capability.name}` : capability.name;
}

function initialCapabilityValue(
  capabilities: CapabilityWithService[],
  name: string | undefined,
  connectionId: string | null | undefined,
  connections: TeamConnection[],
) {
  const connectionService = connections.find(
    (connection) => connection.id === connectionId,
  )?.service;
  const capability =
    capabilities.find(
      (candidate) => candidate.name === name && candidate.service === connectionService,
    ) ??
    capabilities.find((candidate) => candidate.name === name) ??
    capabilities[0];
  return capability ? capabilityOptionValue(capabilities, capability) : "";
}

export function RuleForm({
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
  const [actionValue, setActionValue] = useState(() =>
    initialCapabilityValue(actions, rule?.trigger_kind, rule?.trigger_connection_id, connections),
  );
  const [triggerConnectionId, setTriggerConnectionId] = useState(rule?.trigger_connection_id ?? "");
  const [reactionValue, setReactionValue] = useState(() =>
    initialCapabilityValue(
      reactions,
      rule?.reaction_kind,
      rule?.reaction_connection_id,
      connections,
    ),
  );
  const [reactionConnectionId, setReactionConnectionId] = useState(
    rule?.reaction_connection_id ?? "",
  );
  const initialAction = actions.find(
    (action) => capabilityOptionValue(actions, action) === actionValue,
  );
  const initialReaction = reactions.find(
    (reaction) => capabilityOptionValue(reactions, reaction) === reactionValue,
  );
  const [triggerConfig, setTriggerConfig] = useState<Record<string, string>>(() =>
    catalogValues(initialAction?.fields ?? [], rule?.trigger_config),
  );
  const [reactionConfig, setReactionConfig] = useState<Record<string, string>>(() =>
    catalogValues(initialReaction?.fields ?? [], rule?.reaction_config),
  );
  const createRule = useCreateAutomationRule(teamId);
  const updateRule = useUpdateAutomationRule(teamId);
  const mutation = rule ? updateRule : createRule;

  const selectedAction = actions.find(
    (action) => capabilityOptionValue(actions, action) === actionValue,
  );
  const selectedReaction = reactions.find(
    (reaction) => capabilityOptionValue(reactions, reaction) === reactionValue,
  );
  const isAlertmanagerLifecycleEvent =
    selectedAction?.name === "alert_firing" || selectedAction?.name === "alert_resolved";
  const triggerConnections = connections.filter(
    (connection) => connection.service === selectedAction?.connection_service,
  );
  const isBuiltInAction = selectedAction?.builtIn === true;
  const effectiveTriggerConnectionId = isBuiltInAction
    ? (triggerConnections[0]?.id ?? "")
    : triggerConnectionId;
  const reactionConnections = selectedReaction?.connection_service
    ? connections.filter((connection) => connection.service === selectedReaction.connection_service)
    : [];
  const needsReactionConnection = !!selectedReaction?.connection_service;
  const valid =
    !!name.trim() &&
    !!selectedAction &&
    !!effectiveTriggerConnectionId &&
    !!selectedReaction &&
    catalogFieldsAreValid(selectedAction.fields, triggerConfig) &&
    catalogFieldsAreValid(selectedReaction.fields, reactionConfig) &&
    (!needsReactionConnection || !!reactionConnectionId);

  const selectAction = (nextValue: string) => {
    setActionValue(nextValue);
    const next = actions.find((action) => capabilityOptionValue(actions, action) === nextValue);
    if (
      connections.find((item) => item.id === triggerConnectionId)?.service !==
      next?.connection_service
    ) {
      setTriggerConnectionId("");
    }
    setTriggerConfig(catalogValues(next?.fields ?? []));
  };

  const selectReaction = (nextValue: string) => {
    setReactionValue(nextValue);
    const next = reactions.find(
      (reaction) => capabilityOptionValue(reactions, reaction) === nextValue,
    );
    if (!next?.connection_service) setReactionConnectionId("");
    else if (
      connections.find((item) => item.id === reactionConnectionId)?.service !==
      next.connection_service
    ) {
      setReactionConnectionId("");
    }
    setReactionConfig(catalogValues(next?.fields ?? []));
  };

  const definition = (): AutomationRuleDefinition => ({
    name: name.trim(),
    trigger_connection_id: effectiveTriggerConnectionId,
    trigger_kind: selectedAction?.name ?? "",
    trigger_config: catalogPayload(selectedAction?.fields ?? [], triggerConfig),
    reaction_kind: selectedReaction?.name ?? "",
    reaction_connection_id: needsReactionConnection ? reactionConnectionId : null,
    reaction_config: catalogPayload(selectedReaction?.fields ?? [], reactionConfig),
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

        <fieldset className="space-y-4 pt-2">
          <legend className="text-muted-2 text-[11px] font-semibold tracking-wider uppercase">
            {t("action")}
          </legend>
          <label className="text-text block text-sm font-medium">
            <span>{t("event")}</span>
            <select
              value={actionValue}
              onChange={(event) => selectAction(event.target.value)}
              className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            >
              {actions.map((action) => (
                <option
                  key={`${action.service}:${action.name}`}
                  value={capabilityOptionValue(actions, action)}
                >
                  {action.label}
                </option>
              ))}
            </select>
          </label>
          {!isBuiltInAction ? (
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
          ) : null}
          {triggerConnections.length === 0 ? (
            <Alert tone="warning">{t("missingSourceConnection")}</Alert>
          ) : null}
          {isAlertmanagerLifecycleEvent ? (
            <Alert tone="info" data-testid="alertmanager-lifecycle-contract">
              {t("alertmanagerLifecycleContract")}
            </Alert>
          ) : null}
          {selectedAction?.fields.length ? (
            <div>
              <div className="text-text text-sm font-medium">{t("actionConfiguration")}</div>
              <p className="text-muted mt-1 text-xs">{selectedAction.description}</p>
              <div className="mt-3 grid gap-3 sm:grid-cols-2">
                {selectedAction.fields.map((field) => (
                  <label key={field.name} className="text-muted block text-xs font-medium">
                    <span>{field.label}</span>
                    {field.input_type === "select" ? (
                      <select
                        value={triggerConfig[field.name] ?? ""}
                        onChange={(event) =>
                          setTriggerConfig((current) => ({
                            ...current,
                            [field.name]: event.target.value,
                          }))
                        }
                        className="ow-input mt-1.5 h-9 w-full rounded-md px-3 text-sm"
                        required={field.required}
                      >
                        {!field.required ? <option value="" /> : null}
                        {field.options.map((option) => (
                          <option key={option.value} value={option.value}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <input
                        type={field.input_type}
                        min={field.name === "minutes" ? 5 : undefined}
                        max={field.name === "minutes" ? 1440 : undefined}
                        step={field.name === "minutes" ? 1 : undefined}
                        list={field.name === "timezone" ? "opswarden-timezones" : undefined}
                        value={triggerConfig[field.name] ?? ""}
                        onChange={(event) =>
                          setTriggerConfig((current) => ({
                            ...current,
                            [field.name]: event.target.value,
                          }))
                        }
                        className="ow-input mt-1.5 h-9 w-full rounded-md px-3 text-sm"
                        required={field.required}
                      />
                    )}
                    {field.name === "timezone" ? (
                      <datalist id="opswarden-timezones">
                        <option value="Europe/Paris" />
                        <option value="UTC" />
                      </datalist>
                    ) : null}
                    <span className="mt-1 block font-normal">{field.description}</span>
                  </label>
                ))}
              </div>
            </div>
          ) : null}
        </fieldset>

        <fieldset className="space-y-4 pt-2">
          <legend className="text-muted-2 text-[11px] font-semibold tracking-wider uppercase">
            {t("reaction")}
          </legend>
          <label className="text-text block text-sm font-medium">
            <span>{t("outcome")}</span>
            <select
              value={reactionValue}
              onChange={(event) => selectReaction(event.target.value)}
              className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
            >
              {reactions.map((reaction) => (
                <option
                  key={`${reaction.service}:${reaction.name}`}
                  value={capabilityOptionValue(reactions, reaction)}
                >
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
          {selectedReaction?.fields.length ? (
            <div className="grid gap-3 sm:grid-cols-2">
              {selectedReaction.fields.map((field) => (
                <label key={field.name} className="text-text block text-sm font-medium">
                  <span>{field.label}</span>
                  {field.input_type === "select" ? (
                    <select
                      value={reactionConfig[field.name] ?? ""}
                      onChange={(event) =>
                        setReactionConfig((current) => ({
                          ...current,
                          [field.name]: event.target.value,
                        }))
                      }
                      className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
                      required={field.required}
                    >
                      {!field.required ? <option value="" /> : null}
                      {field.options.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      type={field.input_type}
                      value={reactionConfig[field.name] ?? ""}
                      onChange={(event) =>
                        setReactionConfig((current) => ({
                          ...current,
                          [field.name]: event.target.value,
                        }))
                      }
                      className="ow-input mt-2 h-10 w-full rounded-md px-3 text-sm"
                      required={field.required}
                    />
                  )}
                  <span className="text-muted mt-1 block text-xs font-normal">
                    {field.description}
                  </span>
                </label>
              ))}
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
